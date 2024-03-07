use cursive::align::HAlign;
use cursive::views::{LinearLayout, TextView, Panel, ResizedView};
use cursive::view::SizeConstraint;

use libamdgpu_top::AMDGPU::{DeviceHandle, GPU_INFO, MetricsInfo};
use libamdgpu_top::{AppDeviceInfo, DevicePath, Sampling};

use crate::{TOGGLE_HELP, ToggleOptions, view::*};

use libamdgpu_top::app::{AppAmdgpuTop, AppOption};

pub(crate) struct NewTuiApp {
    pub app_amdgpu_top: AppAmdgpuTop,
    pub index: usize,
    pub grbm_view: PerfCounterView,
    pub grbm2_view: PerfCounterView,
    pub vram_usage_view: VramUsageView,
    pub fdinfo_view: AppTextView,
    pub sensors_view: AppTextView,
    pub gpu_metrics_view: AppTextView,
    pub ecc_view: AppTextView,
    pub no_pc: bool,
}

impl NewTuiApp {
    pub fn new(
        amdgpu_dev: DeviceHandle,
        device_path: DevicePath,
        no_pc: bool,
        index: usize,
    ) -> Option<Self> {
        let app_amdgpu_top = AppAmdgpuTop::new(
            amdgpu_dev,
            device_path,
            &AppOption { pcie_bw: true },
        )?;

        let grbm_view = PerfCounterView::new(&app_amdgpu_top.stat.grbm, index);
        let grbm2_view = PerfCounterView::new(&app_amdgpu_top.stat.grbm2, index);

        Some(Self {
            app_amdgpu_top,
            index,
            grbm_view,
            grbm2_view,
            vram_usage_view: VramUsageView::new(index),
            fdinfo_view: Default::default(),
            sensors_view: Default::default(),
            gpu_metrics_view: Default::default(),
            ecc_view: Default::default(),
            no_pc,
        })
    }

    pub fn layout(&self, title: &str) -> ResizedView<LinearLayout> {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(self.app_amdgpu_top.device_info.info_bar()).center()
                )
                .title(title)
                .title_position(HAlign::Center)
            );

        if !self.no_pc {
            layout.add_child(self.grbm_view.top_view(&self.app_amdgpu_top.stat.grbm, true));
            layout.add_child(self.grbm2_view.top_view(&self.app_amdgpu_top.stat.grbm2, true));
        }

        layout.add_child(self.vram_usage_view.view(&self.app_amdgpu_top.stat.vram_usage));
        layout.add_child(self.fdinfo_view.text.panel("fdinfo"));
        layout.add_child(self.sensors_view.text.panel("Sensors"));

        if self.app_amdgpu_top.stat.memory_error_count.is_some() {
            layout.add_child(self.ecc_view.text.panel("ECC Error Count"));
        }

        if let Some(metrics) = &self.app_amdgpu_top.stat.metrics {
            let title = match metrics.get_header() {
                Some(v) => format!("GPU Metrics v{}.{}", v.format_revision, v.content_revision),
                None => "GPU Metrics".to_string(),
            };

            layout.add_child(self.gpu_metrics_view.text.panel(&title));
        }

        layout.add_child(TextView::new(TOGGLE_HELP));

        ResizedView::new(SizeConstraint::Full, SizeConstraint::Full, layout)
    }

    pub fn update(&mut self, flags: &ToggleOptions, sample: &Sampling) {
        self.app_amdgpu_top.update(sample.to_duration());

        if flags.fdinfo {
            let _ = self.fdinfo_view.print_fdinfo(
                &mut self.app_amdgpu_top.stat.fdinfo,
                flags.fdinfo_sort,
                flags.reverse_sort,
            );
        } else {
            self.fdinfo_view.text.clear();
        }

        self.vram_usage_view.set_value(&self.app_amdgpu_top.stat.vram_usage);

        if flags.sensor {
            let _ = self.sensors_view.print_sensors(&self.app_amdgpu_top.stat.sensors);

            {
                if let Some(arc_pcie_bw) = &self.app_amdgpu_top.stat.arc_pcie_bw {
                    let lock = arc_pcie_bw.try_lock();
                    if let Ok(pcie_bw) = &lock {
                        let _ = self.sensors_view.print_pcie_bw(pcie_bw);
                    }
                }
            }
        } else {
            self.sensors_view.text.clear();
        }

        if let Some(ecc) = &self.app_amdgpu_top.stat.memory_error_count {
            let _ = self.ecc_view.print_memory_error_count(ecc);
        }

        if flags.gpu_metrics {
            if let Some(metrics) = &self.app_amdgpu_top.stat.metrics {
                let _ = self.gpu_metrics_view.print_gpu_metrics(metrics);
            } else {
                self.gpu_metrics_view.text.clear();
            }
        } else {
            self.gpu_metrics_view.text.clear();
        }

        if !self.no_pc {
            self.grbm_view.set_value(&self.app_amdgpu_top.stat.grbm);
            self.grbm2_view.set_value(&self.app_amdgpu_top.stat.grbm2);
        }

        self.sensors_view.text.set();
        self.fdinfo_view.text.set();
        self.ecc_view.text.set();
        self.gpu_metrics_view.text.set();
    }
}

pub trait ListNameInfoBar {
    fn list_name(&self) -> String;
    fn info_bar(&self) -> String;
}

impl ListNameInfoBar for AppDeviceInfo {
    fn list_name(&self) -> String {
        format!("{} ({})", self.marketing_name, self.pci_bus)
    }

    fn info_bar(&self) -> String {
        format!(
            concat!(
                "{mark_name} ({pci}, {did:#06X}:{rid:#04X})\n",
                "{asic}, {gpu_type}, {chip_class},{gfx_ver} {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
                "{vram_type} {vram_bus_width}-bit, {vram_size} MiB, ",
                "{min_memory_clk}-{max_memory_clk} MHz",
            ),
            mark_name = self.marketing_name,
            pci = self.pci_bus,
            did = self.ext_info.device_id(),
            rid = self.ext_info.pci_rev_id(),
            asic = self.ext_info.get_asic_name(),
            gpu_type = if self.ext_info.is_apu() { "APU" } else { "dGPU" },
            chip_class = self.ext_info.get_chip_class(),
            gfx_ver = match &self.gfx_target_version {
                Some(ver) => format!(" {ver},"),
                None => String::new(),
            },
            num_cu = self.ext_info.cu_active_number(),
            min_gpu_clk = self.min_gpu_clk,
            max_gpu_clk = self.max_gpu_clk,
            vram_type = self.ext_info.get_vram_type(),
            vram_bus_width = self.ext_info.vram_bit_width,
            vram_size = self.memory_info.vram.total_heap_size >> 20,
            min_memory_clk = self.min_mem_clk,
            max_memory_clk = self.max_mem_clk,
        )
    }
}
