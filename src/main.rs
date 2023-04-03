use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, CHIP_CLASS, GPU_INFO};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::views::{TextView, LinearLayout, Panel};
use cursive::view::Scrollable;
use cursive::align::HAlign;

mod stat;
mod args;
mod misc;
mod dump_info;
mod json_output;

use stat::FdInfoSortType;

#[derive(Debug, Clone)]
struct ToggleOptions {
    grbm: bool,
    grbm2: bool,
    cp_stat: bool,
    vram: bool,
    sensor: bool,
    high_freq: bool,
    fdinfo: bool,
    fdinfo_sort: FdInfoSortType,
    reverse_sort: bool,
}

impl Default for ToggleOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            grbm2: true,
            cp_stat: true,
            vram: true,
            sensor: true,
            high_freq: false,
            fdinfo: true,
            fdinfo_sort: FdInfoSortType::PID,
            reverse_sort: false,
        }
    }
}

type Opt = Arc<Mutex<ToggleOptions>>;

const TOGGLE_HELP: &str = concat!(
    " (g)rbm g(r)bm2 (c)p_stat \n",
    " (v)ram (f)dinfo se(n)sor (h)igh_freq (q)uit \n",
    " (P): sort_by_pid (M): sort_by_vram (G): sort_by_gfx (R): reverse"
);

fn main() {
    let main_opt = args::MainOpt::parse();
    let device_path = format!("/dev/dri/renderD{}", 128 + main_opt.instance);

    let (amdgpu_dev, major, minor) = {
        use std::fs::File;
        use std::os::fd::IntoRawFd;

        let f = File::open(&device_path).unwrap();

        DeviceHandle::init(f.into_raw_fd()).unwrap()
    };

    if main_opt.dump {
        dump_info::dump(&amdgpu_dev, major, minor);
        return;
    }

    if main_opt.json_output {
        let self_pid: i32 = main_opt.pid.unwrap_or(procfs::process::Process::myself().unwrap().pid());
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

    let (min_gpu_clk, min_memory_clk) = misc::get_min_clk(&amdgpu_dev, &pci_bus);
    let mark_name = amdgpu_dev.get_marketing_name().unwrap_or("".to_string());
    let info_bar = format!(
        concat!(
            "{mark_name} ({did:#06X}:{rid:#04X})\n",
            "{asic}, {gpu_type}, {chip_class}, {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
            "{vram_type} {vram_bus_width}-bit, {vram_size} MiB, ",
            "{min_memory_clk}-{max_memory_clk} MHz",
        ),
        mark_name = mark_name,
        did = ext_info.device_id(),
        rid = ext_info.pci_rev_id(),
        asic = ext_info.get_asic_name(),
        gpu_type = if ext_info.is_apu() { "APU" } else { "dGPU" },
        chip_class = chip_class,
        num_cu = ext_info.cu_active_number(),
        min_gpu_clk = min_gpu_clk,
        max_gpu_clk = ext_info.max_engine_clock().saturating_div(1000),
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
        vram_size = memory_info.vram.total_heap_size >> 20,
        min_memory_clk = min_memory_clk,
        max_memory_clk = ext_info.max_memory_clock().saturating_div(1000),
    );

    let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
        stat::GFX10_GRBM_INDEX
    } else {
        stat::GRBM_INDEX
    };

    let mut grbm = stat::PerfCounter::new(stat::PCType::GRBM, grbm_index);
    let mut grbm2 = stat::PerfCounter::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);
    let mut cp_stat = stat::PerfCounter::new(stat::PCType::CP_STAT, stat::CP_STAT_INDEX);
    let mut vram = stat::VRAM_INFO::new(&memory_info);

    let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
    let mut sample = Sampling::low();
    let mut fdinfo = stat::FdInfoView::new(sample.to_duration());
    let mut sensor = stat::Sensor::new(&pci_bus);

    let mut toggle_opt = ToggleOptions::default();

    {   // check register offset
        toggle_opt.grbm = grbm.pc_type.check_reg_offset(&amdgpu_dev);
        toggle_opt.grbm2 = grbm2.pc_type.check_reg_offset(&amdgpu_dev);
        [toggle_opt.cp_stat, _] = [false, cp_stat.pc_type.check_reg_offset(&amdgpu_dev)];

        // fill
        {
            stat::update_index(&mut proc_index, &device_path);
            fdinfo.print(&proc_index, &FdInfoSortType::PID, false);
            fdinfo.text.set();
        }
        {
            vram.print();
            vram.text.set();
        }
        {
            sensor.print(&amdgpu_dev);
            sensor.text.set();
        }
    }

    let mut siv = cursive::default();
    {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(info_bar).center()
                )
                .title(env!("CARGO_PKG_NAME"))
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
            layout.add_child(cp_stat.top_view(toggle_opt.cp_stat));
            siv.add_global_callback('c', cp_stat.pc_type.cb());
        }
        {
            layout.add_child(vram.text.panel("Memory Usage"));
            siv.add_global_callback('v', stat::VRAM_INFO::cb);
        }
        {
            layout.add_child(fdinfo.text.panel("fdinfo"));
            siv.add_global_callback('f', stat::FdInfoView::cb);
            siv.add_global_callback('R', stat::FdInfoView::cb_reverse_sort);
            siv.add_global_callback('P', stat::FdInfoView::cb_sort_by_pid);
            siv.add_global_callback('M', stat::FdInfoView::cb_sort_by_vram);
            siv.add_global_callback('G', stat::FdInfoView::cb_sort_by_gfx);
        }
        {
            layout.add_child(sensor.text.panel("Sensors"));
            siv.add_global_callback('n', stat::Sensor::cb);
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

    {
        let index = share_proc_index.clone();
        let mut buf_index: Vec<stat::ProcInfo> = Vec::new();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(5));

                stat::update_index(&mut buf_index, &device_path);

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
                if flags.cp_stat {
                    cp_stat.read_reg(&amdgpu_dev);
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
                vram.update_usage(&amdgpu_dev);
                vram.print();
            } else {
                vram.text.clear();
            }

            if flags.sensor {
                sensor.print(&amdgpu_dev);
            } else {
                sensor.text.clear();
            }

            if flags.fdinfo {
                let lock = index.try_lock();
                if let Ok(vec_info) = lock {
                    fdinfo.print(&vec_info, &flags.fdinfo_sort, flags.reverse_sort);
                    fdinfo.interval = sample.to_duration();
                } else {
                    fdinfo.interval += sample.to_duration();
                }
            } else {
                fdinfo.text.clear();
            }

            grbm.dump();
            grbm2.dump();
            cp_stat.dump();

            vram.text.set();
            fdinfo.text.set();
            sensor.text.set();

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
