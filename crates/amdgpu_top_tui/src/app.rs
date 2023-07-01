use std::sync::{Arc, Mutex};
use cursive::align::HAlign;
use cursive::views::{LinearLayout, TextView, Panel, ResizedView};
use cursive::view::SizeConstraint;

use libamdgpu_top::AMDGPU::{DeviceHandle, drm_amdgpu_info_device, drm_amdgpu_memory_info, GPU_INFO};
use libamdgpu_top::{DevicePath, PCI, Sampling};
use libamdgpu_top::stat::{self, PcieBw, ProcInfo, Sensors};

use crate::{TOGGLE_HELP, ToggleOptions, view::*};

pub(crate) struct TuiApp {
    pub amdgpu_dev: DeviceHandle,
    pub device_path: DevicePath,
    pub instance: u32,
    pub list_name: String,
    pub device_info: String,
    pub grbm: PerfCounterView,
    pub grbm2: PerfCounterView,
    pub fdinfo: FdInfoView,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub gpu_metrics: GpuMetricsView,
    pub vram_usage: VramUsageView,
    pub support_pcie_bw: bool,
    pub sensors: SensorsView,
    pub arc_pcie_bw: Arc<Mutex<PcieBw>>,
}

impl TuiApp {
    pub fn new(
        amdgpu_dev: DeviceHandle,
        device_path: &DevicePath,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
    ) -> Self {
        let instance = device_path.get_instance_number().unwrap();
        let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
        let sensors = Sensors::new(&amdgpu_dev, &pci_bus, ext_info.get_asic_name());
        let device_info = info_bar(
            &amdgpu_dev,
            ext_info,
            &pci_bus,
            memory_info.vram.total_heap_size,
        );
        let sensors_view = SensorsView::new_with_sensors(sensors);
        let list_name = format!("{} ({pci_bus})", amdgpu_dev.get_marketing_name_or_default());
        let chip_class = ext_info.get_chip_class();

        let grbm = PerfCounterView::new_with_chip_class(stat::PCType::GRBM, chip_class);
        let grbm2 = PerfCounterView::new_with_chip_class(stat::PCType::GRBM2, chip_class);
        let vram_usage = VramUsageView::new(memory_info);

        let mut fdinfo = FdInfoView::new(
            Sampling::default().to_duration(),
            libamdgpu_top::has_vcn_unified(&amdgpu_dev),
        );

        let arc_proc_index = {
            let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
            stat::update_index(&mut proc_index, device_path);

            fdinfo.print(&proc_index, &Default::default(), false).unwrap();
            fdinfo.text.set();

            Arc::new(Mutex::new(proc_index))
        };

        let gpu_metrics = GpuMetricsView::new(&amdgpu_dev);
        let (support_pcie_bw, arc_pcie_bw) = {
            let pcie_bw = PcieBw::new(pci_bus.get_sysfs_path());

            (pcie_bw.exists.clone(), Arc::new(Mutex::new(pcie_bw)))
        };

        Self {
            amdgpu_dev,
            device_path: device_path.clone(),
            instance,
            list_name,
            device_info,
            grbm,
            grbm2,
            arc_proc_index,
            fdinfo,
            vram_usage,
            sensors: sensors_view,
            support_pcie_bw,
            arc_pcie_bw,
            gpu_metrics,
        }
    }

    pub fn fill(&mut self, toggle_opt: &mut ToggleOptions) {
        if self.gpu_metrics.update_metrics(&self.amdgpu_dev).is_ok() {
            toggle_opt.gpu_metrics = true;
            self.gpu_metrics.print().unwrap();
            self.gpu_metrics.text.set();
        }

        self.vram_usage.set_value();

        self.sensors.update(&self.amdgpu_dev);
        self.sensors.print().unwrap();
        {
            if let Ok(pcie_bw) = self.arc_pcie_bw.lock() {
                if pcie_bw.exists {
                    self.sensors.print_pcie_bw(&pcie_bw).unwrap();
                }
            }
        }
        self.sensors.text.set();
    }

    pub fn layout(&self, title: &str, toggle_opt: &ToggleOptions) -> ResizedView<LinearLayout> {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(&self.device_info).center()
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
        }
        layout.add_child(TextView::new(TOGGLE_HELP));

        ResizedView::new(SizeConstraint::Free, SizeConstraint::Full, layout)
    }

    pub fn update_pc(&mut self, flags: &ToggleOptions) {
        // high frequency accesses to registers can cause high GPU clocks
        if flags.grbm {
            self.grbm.pc.read_reg(&self.amdgpu_dev);
        }
        if flags.grbm2 {
            self.grbm2.pc.read_reg(&self.amdgpu_dev);
        }
    }

    pub fn update(&mut self, flags: &ToggleOptions, sample: &Sampling) {
        if flags.vram {
            self.vram_usage.update_usage(&self.amdgpu_dev);
        }

        if flags.sensor {
            self.sensors.update(&self.amdgpu_dev);
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
            if self.gpu_metrics.update_metrics(&self.amdgpu_dev).is_ok() {
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

fn info_bar(
    amdgpu_dev: &DeviceHandle,
    ext_info: &drm_amdgpu_info_device,
    pci_bus: &PCI::BUS_INFO,
    total_vram_size: u64,
) -> String {
    let (min_gpu_clk, max_gpu_clk) = amdgpu_dev.get_min_max_gpu_clock()
        .unwrap_or_else(|| (0, (ext_info.max_engine_clock() / 1000) as u32));
    let (min_mem_clk, max_mem_clk) = amdgpu_dev.get_min_max_memory_clock()
        .unwrap_or_else(|| (0, (ext_info.max_memory_clock() / 1000) as u32));

    format!(
        concat!(
            "{mark_name} ({pci}, {did:#06X}:{rid:#04X})\n",
            "{asic}, {gpu_type}, {chip_class}, {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
            "{vram_type} {vram_bus_width}-bit, {vram_size} MiB, ",
            "{min_memory_clk}-{max_memory_clk} MHz",
        ),
        mark_name = amdgpu_dev.get_marketing_name_or_default(),
        pci = pci_bus,
        did = ext_info.device_id(),
        rid = ext_info.pci_rev_id(),
        asic = ext_info.get_asic_name(),
        gpu_type = if ext_info.is_apu() { "APU" } else { "dGPU" },
        chip_class = ext_info.get_chip_class(),
        num_cu = ext_info.cu_active_number(),
        min_gpu_clk = min_gpu_clk,
        max_gpu_clk = max_gpu_clk,
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
        vram_size = total_vram_size >> 20,
        min_memory_clk = min_mem_clk,
        max_memory_clk = max_mem_clk,
    )
}
