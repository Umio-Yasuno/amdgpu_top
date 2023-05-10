use libamdgpu_top::AMDGPU::{CHIP_CLASS, DeviceHandle, drm_amdgpu_info_device, drm_amdgpu_memory_info, GPU_INFO};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::views::{TextView, LinearLayout, Panel};
use cursive::view::Scrollable;
use cursive::align::HAlign;

use libamdgpu_top::{stat, DevicePath, Sampling};
use stat::{PcieBw, PCType, ProcInfo};

mod view;
use view::*;

#[derive(Clone)]
pub(crate) struct TuiApp {
    pub grbm: PerfCounterView,
    pub grbm2: PerfCounterView,
    pub fdinfo: FdInfoView,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub gpu_metrics: GpuMetricsView,
    pub vram_usage: VramUsageView,
    pub sensors: SensorsView,
    pub arc_pcie_bw: Arc<Mutex<PcieBw>>,
}

impl TuiApp {
    fn new(
        amdgpu_dev: &DeviceHandle,
        device_path: &DevicePath,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
    ) -> Self {
        let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
        let chip_class = ext_info.get_chip_class();

        let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
            stat::GFX10_GRBM_INDEX
        } else {
            stat::GRBM_INDEX
        };

        let grbm = PerfCounterView::new(stat::PCType::GRBM, grbm_index);
        let grbm2 = PerfCounterView::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);
        let vram_usage = VramUsageView::new(&memory_info);

        let mut fdinfo = FdInfoView::new(Sampling::default().to_duration());

        let arc_proc_index = {
            let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
            stat::update_index(&mut proc_index, &device_path, 1);

            fdinfo.print(&proc_index, &Default::default(), false).unwrap();
            fdinfo.text.set();

            Arc::new(Mutex::new(proc_index))
        };

        let gpu_metrics = GpuMetricsView::new(&amdgpu_dev);
        let sensors = SensorsView::new(&amdgpu_dev, &pci_bus);
        let arc_pcie_bw = Arc::new(Mutex::new(PcieBw::new(pci_bus.get_sysfs_path())));

        Self {
            grbm,
            grbm2,
            arc_proc_index,
            fdinfo,
            vram_usage,
            sensors,
            arc_pcie_bw,
            gpu_metrics,
        }
    }

    fn fill(&mut self, amdgpu_dev: &DeviceHandle, toggle_opt: &mut ToggleOptions) {
        if self.gpu_metrics.update_metrics(amdgpu_dev).is_ok() {
            toggle_opt.gpu_metrics = true;
            self.gpu_metrics.print().unwrap();
            self.gpu_metrics.text.set();
        }

        self.vram_usage.set_value();

        self.sensors.update(amdgpu_dev);
        self.sensors.print().unwrap();
        {
            if let Ok(pcie_bw) = self.arc_pcie_bw.lock() {
                if pcie_bw.exists {
                    toggle_opt.pcie_bw = true;
                    self.sensors.print_pcie_bw(&pcie_bw).unwrap();
                }
            }
        }
        self.sensors.text.set();
    }

    fn layout(&self, title: &str, device_info: &str, toggle_opt: &ToggleOptions) -> LinearLayout {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(device_info).center()
                )
                .title(title)
                .title_position(HAlign::Center)
            );

        layout.add_child(self.grbm.top_view(toggle_opt.grbm));
        layout.add_child(self.grbm2.top_view(toggle_opt.grbm2));
        layout.add_child(self.vram_usage.view());
        layout.add_child(self.fdinfo.text.panel("fdinfo"));
        layout.add_child(self.sensors.text.panel("Sensors"));

        if toggle_opt.gpu_metrics {
            let title = match self.gpu_metrics.version() {
                Some(v) => format!("GPU Metrics v{}.{}", v.0, v.1),
                None => "GPU Metrics".to_string(),
            };

            layout.add_child(self.gpu_metrics.text.panel(&title));
            // siv.add_global_callback('m', GpuMetricsView::cb);
        }
        layout.add_child(TextView::new(TOGGLE_HELP));

        layout
    }

    fn update(&mut self, amdgpu_dev: &DeviceHandle, flags: &ToggleOptions, sample: &Sampling) {
        for _ in 0..sample.count {
            // high frequency accesses to registers can cause high GPU clocks
            if flags.grbm {
                self.grbm.pc.read_reg(&amdgpu_dev);
            }
            if flags.grbm2 {
                self.grbm2.pc.read_reg(&amdgpu_dev);
            }

            std::thread::sleep(sample.delay);
        }

        if flags.vram {
            self.vram_usage.update_usage(&amdgpu_dev);
        }

        if flags.sensor {
            self.sensors.update(&amdgpu_dev);
            self.sensors.print().unwrap();

            if let Ok(pcie_bw) = self.arc_pcie_bw.try_lock() {
                if pcie_bw.exists {
                    self.sensors.print_pcie_bw(&pcie_bw).unwrap();
                }
            }
        } else {
            self.sensors.text.clear();
        }

        if flags.fdinfo {
            let lock = self.arc_proc_index.try_lock();
            if let Ok(vec_info) = lock {
                self.fdinfo.print(&vec_info, &flags.fdinfo_sort, flags.reverse_sort).unwrap();
                self.fdinfo.stat.interval = sample.to_duration();
            } else {
                self.fdinfo.stat.interval += sample.to_duration();
            }
        } else {
            self.fdinfo.text.clear();
        }

        if flags.gpu_metrics {
            if self.gpu_metrics.update_metrics(&amdgpu_dev).is_ok() {
                self.gpu_metrics.print().unwrap();
            }
        } else {
            self.gpu_metrics.text.clear();
        }

        self.grbm.dump();
        self.grbm2.dump();

        self.vram_usage.set_value();
        self.fdinfo.text.set();
        self.sensors.text.set();
        self.gpu_metrics.text.set();
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
    fdinfo_sort: stat::FdInfoSortType,
    reverse_sort: bool,
    gpu_metrics: bool,
    pcie_bw: bool,
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
            fdinfo_sort: Default::default(),
            reverse_sort: false,
            gpu_metrics: false,
            pcie_bw: false,
        }
    }
}

type Opt = Arc<Mutex<ToggleOptions>>;

const TOGGLE_HELP: &str = concat!(
    " (g)rbm g(r)bm2 (v)ram_usage (f)dinfo \n",
    " se(n)sor (m)etrics (h)igh_freq (q)uit \n",
    " (P): sort_by_pid (V): sort_by_vram (G): sort_by_gfx\n (M): sort_by_media (R): reverse"
);

pub fn run(
    title: &str,
    device_path: DevicePath,
    amdgpu_dev: DeviceHandle,
    // device_path_list: &[DevicePath],
    interval: u64,
) {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();

    let mut toggle_opt = ToggleOptions::default();
    let device_info = info_bar(&amdgpu_dev, &ext_info, memory_info.vram.total_heap_size);

    let mut app = TuiApp::new(&amdgpu_dev, &device_path, &ext_info, &memory_info);
    app.fill(&amdgpu_dev, &mut toggle_opt);

    let mut siv = cursive::default();
    {
        siv.add_global_callback('g', pc_type_cb(&PCType::GRBM));
        siv.add_global_callback('r', pc_type_cb(&PCType::GRBM2));
        siv.add_global_callback('v', VramUsageView::cb);
        siv.add_global_callback('f', FdInfoView::cb);
        siv.add_global_callback('R', FdInfoView::cb_reverse_sort);
        siv.add_global_callback('P', FdInfoView::cb_sort_by_pid);
        siv.add_global_callback('V', FdInfoView::cb_sort_by_vram);
        siv.add_global_callback('G', FdInfoView::cb_sort_by_gfx);
        siv.add_global_callback('M', FdInfoView::cb_sort_by_media);
        siv.add_global_callback('n', SensorsView::cb);
        siv.add_global_callback('m', GpuMetricsView::cb);
        siv.add_global_callback('q', cursive::Cursive::quit);
        siv.add_global_callback('h', |siv: &mut cursive::Cursive| {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.high_freq ^= true;
        });

        siv.add_layer(
            app.layout(title, &device_info, &toggle_opt)
                .scrollable()
                .scroll_y(true)
        );
    }

    if toggle_opt.pcie_bw {
        if let Ok(pcie_bw) = app.arc_pcie_bw.lock() {
            let arc_pcie_bw = app.arc_pcie_bw.clone();
            let mut buf_pcie_bw = pcie_bw.clone();

            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(500)); // wait for user input
                buf_pcie_bw.update(); // msleep(1000)

                let lock = arc_pcie_bw.lock();
                if let Ok(mut pcie_bw) = lock {
                    *pcie_bw = buf_pcie_bw.clone();
                }
            });
        }
    }

    {
        let index = app.arc_proc_index.clone();
        let mut buf_index: Vec<ProcInfo> = Vec::new();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(interval));

            stat::update_index(&mut buf_index, &device_path, 1);

            let lock = index.lock();
            if let Ok(mut index) = lock {
                *index = buf_index.clone();
            }
        });
    }

    let mut flags = toggle_opt.clone();
    let toggle_opt = Arc::new(Mutex::new(toggle_opt));

    siv.set_user_data(toggle_opt.clone());

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || loop {
        {
            let lock = toggle_opt.try_lock();
            if let Ok(opt) = lock {
                flags = opt.clone();
            }
        }

        let sample = if flags.high_freq {
            Sampling::high()
        } else {
            Sampling::low()
        };

        app.update(&amdgpu_dev, &flags, &sample);

        cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
    });

    siv.run();
}

pub fn info_bar(
    amdgpu_dev: &DeviceHandle,
    ext_info: &drm_amdgpu_info_device,
    vram_size: u64,
) -> String {
    let chip_class = ext_info.get_chip_class();

    let (min_gpu_clk, max_gpu_clk) = amdgpu_dev.get_min_max_gpu_clock().unwrap_or((0, 0));
    let (min_mem_clk, max_mem_clk) = amdgpu_dev.get_min_max_memory_clock().unwrap_or((0, 0));
    let mark_name = amdgpu_dev.get_marketing_name().unwrap_or("".to_string());

    format!(
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
        max_gpu_clk = max_gpu_clk,
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
        vram_size = vram_size >> 20,
        min_memory_clk = min_mem_clk,
        max_memory_clk = max_mem_clk,
    )
}
