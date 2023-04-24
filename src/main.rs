use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, CHIP_CLASS, GPU_INFO};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::views::{TextView, LinearLayout, Panel};
use cursive::view::Scrollable;
use cursive::align::HAlign;

#[cfg(feature = "egui")]
mod gui;

mod stat;
mod args;
mod misc;
mod dump_info;
mod json_output;

use stat::FdInfoSortType;

#[derive(Debug, Clone)]
pub struct DevicePath {
    pub render: String,
    pub card: String,
}

impl DevicePath {
    pub fn new(instance: u32) -> Self {
        Self {
            render: format!("/dev/dri/renderD{}", 128 + instance),
            card: format!("/dev/dri/card{}", instance),
        }
    }

    pub fn init_device_handle(&self) -> DeviceHandle {
        let (amdgpu_dev, _major, _minor) = {
            use std::os::fd::IntoRawFd;
            use std::fs::OpenOptions;

            // need write option for GUI context
            // https://gitlab.freedesktop.org/mesa/mesa/-/issues/2424
            let f = OpenOptions::new().read(true).write(true).open(&self.render)
                .unwrap_or_else(|err| {
                    eprintln!("{err}");
                    eprintln!("render_path = {}", self.render);
                    eprintln!("card_path = {}", self.card);
                    panic!();
                }
            );
            DeviceHandle::init(f.into_raw_fd()).unwrap()
        };

        amdgpu_dev
    }
}

#[derive(Debug, Clone)]
struct ToggleOptions {
    grbm: bool,
    grbm2: bool,
    vram: bool,
    sensor: bool,
    high_freq: bool,
    fdinfo: bool,
    fdinfo_sort: FdInfoSortType,
    reverse_sort: bool,
    gpu_metrics: bool,
}

impl Default for ToggleOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            grbm2: true,
            vram: true,
            sensor: true,
            high_freq: false,
            fdinfo: true,
            fdinfo_sort: FdInfoSortType::VRAM,
            reverse_sort: false,
            gpu_metrics: false,
        }
    }
}

type Opt = Arc<Mutex<ToggleOptions>>;

const TOGGLE_HELP: &str = concat!(
    " (g)rbm g(r)bm2 (v)ram_usage (f)dinfo \n",
    " se(n)sor (m)etrics (h)igh_freq (q)uit \n",
    " (P): sort_by_pid (V): sort_by_vram (G): sort_by_gfx\n (M): sort_by_media (R): reverse"
);

fn main() {
    let main_opt = args::MainOpt::parse();
    let self_pid = stat::get_self_pid().unwrap_or(0);

    #[cfg(feature = "egui")]
    if main_opt.gui {
        gui::egui_run(main_opt.instance, main_opt.update_process_index, self_pid);
        return;
    }

    let device_path = DevicePath::new(main_opt.instance);
    let amdgpu_dev = device_path.init_device_handle();

    if main_opt.dump {
        dump_info::dump(&amdgpu_dev);
        return;
    }

    if main_opt.json_output {
        let Some(self_pid) = main_opt.pid else {
            eprintln!("PID is not specified.");
            return;
        };

        if let Err(err) = json_output::print(
            &amdgpu_dev,
            &device_path,
            main_opt.refresh_period,
            self_pid
        ) {
            eprintln!("Error: {err}");
        }
        return;
    }

    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let chip_class = ext_info.get_chip_class();

    let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
        stat::GFX10_GRBM_INDEX
    } else {
        stat::GRBM_INDEX
    };

    let mut grbm = stat::PerfCounter::new(stat::PCType::GRBM, grbm_index);
    let mut grbm2 = stat::PerfCounter::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);
    let mut vram_usage = stat::VramUsageView::new(&memory_info);

    let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
    let mut sample = Sampling::low();
    let mut fdinfo = stat::FdInfoView::new(sample.to_duration());

    let pcie_bw = stat::PcieBw::new(pci_bus.get_sysfs_path());
    let share_pcie_bw = Arc::new(Mutex::new(pcie_bw.clone()));

    let mut sensor = stat::Sensor::new(&pci_bus);
    let mut metrics = stat::GpuMetricsView::new(&amdgpu_dev);

    let mut toggle_opt = ToggleOptions::default();

    {   // check register offset
        toggle_opt.grbm = grbm.pc_type.check_reg_offset(&amdgpu_dev);
        toggle_opt.grbm2 = grbm2.pc_type.check_reg_offset(&amdgpu_dev);

        if metrics.update_metrics(&amdgpu_dev).is_ok() {
            toggle_opt.gpu_metrics = true;
            metrics.print().unwrap();
            metrics.text.set();
        }

        vram_usage.set_value();

        // fill
        {
            stat::update_index(&mut proc_index, &device_path, self_pid);
            fdinfo.print(&proc_index, &toggle_opt.fdinfo_sort, false).unwrap();
            fdinfo.text.set();
        }
        {
            sensor.print(&amdgpu_dev).unwrap();
            if pcie_bw.exists {
                sensor.print_pcie_bw(&pcie_bw).unwrap();
            }
            sensor.text.set();
        }
    }

    let mut siv = cursive::default();
    {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(misc::info_bar(&amdgpu_dev, &ext_info)).center()
                )
                .title(concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION")))
                .title_position(HAlign::Center)
            );

        if toggle_opt.grbm {
            layout.add_child(grbm.top_view(toggle_opt.grbm));
            siv.add_global_callback('g', grbm.pc_type.cb());
        }
        if toggle_opt.grbm2 {
            layout.add_child(grbm2.top_view(toggle_opt.grbm2));
            siv.add_global_callback('r', grbm2.pc_type.cb());
        }
        {
            layout.add_child(vram_usage.view());
            siv.add_global_callback('v', stat::VramUsageView::cb);
        }
        {
            layout.add_child(fdinfo.text.panel("fdinfo"));
            siv.add_global_callback('f', stat::FdInfoView::cb);
            siv.add_global_callback('R', stat::FdInfoView::cb_reverse_sort);
            siv.add_global_callback('P', stat::FdInfoView::cb_sort_by_pid);
            siv.add_global_callback('V', stat::FdInfoView::cb_sort_by_vram);
            siv.add_global_callback('G', stat::FdInfoView::cb_sort_by_gfx);
            siv.add_global_callback('M', stat::FdInfoView::cb_sort_by_media);
        }
        {
            layout.add_child(sensor.text.panel("Sensors"));
            siv.add_global_callback('n', stat::Sensor::cb);
        }
        if toggle_opt.gpu_metrics {
            let title = match metrics.version() {
                Some(v) => format!("GPU Metrics v{}.{}", v.0, v.1),
                None => "GPU Metrics".to_string(),
            };

            layout.add_child(metrics.text.panel(&title));
            siv.add_global_callback('m', stat::GpuMetricsView::cb);
        }
        layout.add_child(TextView::new(TOGGLE_HELP));

        siv.add_layer(
            layout
                .scrollable()
                .scroll_y(true)
        );
    }

    let mut flags = toggle_opt.clone();
    let toggle_opt = Arc::new(Mutex::new(toggle_opt));
    siv.set_user_data(toggle_opt.clone());
    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('h', Sampling::cb);

    let share_proc_index = Arc::new(Mutex::new(proc_index));
    let cb_sink = siv.cb_sink().clone();

    if pcie_bw.exists {
        let share_pcie_bw = share_pcie_bw.clone();
        let mut buf_pcie_bw = pcie_bw.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1)); // wait for user input
                buf_pcie_bw.update(); // msleep(1000)

                let lock = share_pcie_bw.lock();
                if let Ok(mut share_pcie_bw) = lock {
                    *share_pcie_bw = buf_pcie_bw.clone();
                }
            }
        });
    }

    {
        let interval = main_opt.update_process_index;
        let index = share_proc_index.clone();
        let mut buf_index: Vec<stat::ProcInfo> = Vec::new();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(interval));

                stat::update_index(&mut buf_index, &device_path, self_pid);

                let lock = index.lock();
                if let Ok(mut index) = lock {
                    *index = buf_index.clone();
                }
            }
        });
    }

    std::thread::spawn(move || {
        let index = share_proc_index.clone();

        loop {
            for _ in 0..sample.count {
                // high frequency accesses to registers can cause high GPU clocks
                if flags.grbm {
                    grbm.read_reg(&amdgpu_dev);
                }
                if flags.grbm2 {
                    grbm2.read_reg(&amdgpu_dev);
                }

                std::thread::sleep(sample.delay);
            }

            {
                let lock = toggle_opt.try_lock();
                if let Ok(opt) = lock {
                    flags = opt.clone();
                }
            }

            sample = if flags.high_freq {
                Sampling::high()
            } else {
                Sampling::low()
            };

            if flags.vram {
                vram_usage.update_usage(&amdgpu_dev);
            }

            if flags.sensor {
                sensor.print(&amdgpu_dev).unwrap();

                if pcie_bw.exists {
                    let lock = share_pcie_bw.try_lock();
                    if let Ok(p) = lock {
                        sensor.print_pcie_bw(&p).unwrap();
                    }
                }
            } else {
                sensor.text.clear();
            }

            if flags.fdinfo {
                let lock = index.try_lock();
                if let Ok(vec_info) = lock {
                    fdinfo.print(&vec_info, &flags.fdinfo_sort, flags.reverse_sort).unwrap();
                    fdinfo.interval = sample.to_duration();
                } else {
                    fdinfo.interval += sample.to_duration();
                }
            } else {
                fdinfo.text.clear();
            }

            if flags.gpu_metrics {
                if metrics.update_metrics(&amdgpu_dev).is_ok() {
                    metrics.print().unwrap();
                }
            } else {
                metrics.text.clear();
            }

            grbm.dump();
            grbm2.dump();

            vram_usage.set_value();
            fdinfo.text.set();
            sensor.text.set();
            metrics.text.set();

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
    });

    siv.run();
}

struct Sampling {
    count: usize,
    delay: Duration,
}

impl Sampling {
    const fn low() -> Self {
        Self {
            count: 100,
            delay: Duration::from_millis(10),
        }
    }

    const fn high() -> Self {
        Self {
            count: 100,
            delay: Duration::from_millis(1),
        }
    }

    fn to_duration(&self) -> Duration {
        self.delay * self.count as u32
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.high_freq ^= true;
        }
    }
}
