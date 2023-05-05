use libamdgpu_top::AMDGPU::{CHIP_CLASS, DeviceHandle, drm_amdgpu_info_device, GPU_INFO};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::views::{TextView, LinearLayout, Panel};
use cursive::view::Scrollable;
use cursive::align::HAlign;

use libamdgpu_top::{stat, DevicePath, Sampling};

mod view;
use view::*;

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
    let self_pid = stat::get_self_pid().unwrap_or(0);

    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let chip_class = ext_info.get_chip_class();

    let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
        stat::GFX10_GRBM_INDEX
    } else {
        stat::GRBM_INDEX
    };

    let mut grbm = PerfCounterView::new(stat::PCType::GRBM, grbm_index);
    let mut grbm2 = PerfCounterView::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);
    let mut vram_usage = VramUsageView::new(&memory_info);

    let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
    let mut sample = Sampling::low();
    let mut fdinfo = FdInfoView::new(sample.to_duration());

    let pcie_bw = stat::PcieBw::new(pci_bus.get_sysfs_path());
    let share_pcie_bw = Arc::new(Mutex::new(pcie_bw.clone()));

    let mut sensors_view = SensorsView::new(&amdgpu_dev, &pci_bus);
    let mut metrics = GpuMetricsView::new(&amdgpu_dev);

    let mut toggle_opt = ToggleOptions::default();

    {   // check register offset
        toggle_opt.grbm = grbm.pc.pc_type.check_reg_offset(&amdgpu_dev);
        toggle_opt.grbm2 = grbm2.pc.pc_type.check_reg_offset(&amdgpu_dev);

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
            sensors_view.update(&amdgpu_dev);
            sensors_view.print().unwrap();
            if pcie_bw.exists {
                sensors_view.print_pcie_bw(&pcie_bw).unwrap();
            }
            sensors_view.text.set();
        }
    }

    let mut siv = cursive::default();
    {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(info_bar(&amdgpu_dev, &ext_info)).center()
                )
                .title(title)
                .title_position(HAlign::Center)
            );

        if toggle_opt.grbm {
            layout.add_child(grbm.top_view(toggle_opt.grbm));
            siv.add_global_callback('g', grbm.cb());
        }
        if toggle_opt.grbm2 {
            layout.add_child(grbm2.top_view(toggle_opt.grbm2));
            siv.add_global_callback('r', grbm2.cb());
        }
        {
            layout.add_child(vram_usage.view());
            siv.add_global_callback('v', VramUsageView::cb);
        }
        {
            layout.add_child(fdinfo.text.panel("fdinfo"));
            siv.add_global_callback('f', FdInfoView::cb);
            siv.add_global_callback('R', FdInfoView::cb_reverse_sort);
            siv.add_global_callback('P', FdInfoView::cb_sort_by_pid);
            siv.add_global_callback('V', FdInfoView::cb_sort_by_vram);
            siv.add_global_callback('G', FdInfoView::cb_sort_by_gfx);
            siv.add_global_callback('M', FdInfoView::cb_sort_by_media);
        }
        {
            layout.add_child(sensors_view.text.panel("Sensors"));
            siv.add_global_callback('n', SensorsView::cb);
        }
        if toggle_opt.gpu_metrics {
            let title = match metrics.version() {
                Some(v) => format!("GPU Metrics v{}.{}", v.0, v.1),
                None => "GPU Metrics".to_string(),
            };

            layout.add_child(metrics.text.panel(&title));
            siv.add_global_callback('m', GpuMetricsView::cb);
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
    siv.add_global_callback('h', |siv: &mut cursive::Cursive| {
        let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
        opt.high_freq ^= true;
    });

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
        // let interval = main_opt.update_process_index;
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
                    grbm.pc.read_reg(&amdgpu_dev);
                }
                if flags.grbm2 {
                    grbm2.pc.read_reg(&amdgpu_dev);
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
                sensors_view.update(&amdgpu_dev);
                sensors_view.print().unwrap();

                if pcie_bw.exists {
                    let lock = share_pcie_bw.try_lock();
                    if let Ok(p) = lock {
                        sensors_view.print_pcie_bw(&p).unwrap();
                    }
                }
            } else {
                sensors_view.text.clear();
            }

            if flags.fdinfo {
                let lock = index.try_lock();
                if let Ok(vec_info) = lock {
                    fdinfo.print(&vec_info, &flags.fdinfo_sort, flags.reverse_sort).unwrap();
                    fdinfo.stat.interval = sample.to_duration();
                } else {
                    fdinfo.stat.interval += sample.to_duration();
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
            sensors_view.text.set();
            metrics.text.set();

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
    });

    siv.run();
}

pub fn info_bar(amdgpu_dev: &DeviceHandle, ext_info: &drm_amdgpu_info_device) -> String {
    let memory_info = amdgpu_dev.memory_info().unwrap();
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
        vram_size = memory_info.vram.total_heap_size >> 20,
        min_memory_clk = min_mem_clk,
        max_memory_clk = max_mem_clk,
    )
}
