use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::ops::Range;
use std::path::PathBuf;
use eframe::egui;
use egui::{FontFamily, FontId, util::History};

use libamdgpu_top::AMDGPU::{
    DeviceHandle,
    GpuMetrics,
    MetricsInfo,
    GPU_INFO,
};
use libamdgpu_top::{AppDeviceInfo, DevicePath, Sampling, VramUsage};
use libamdgpu_top::stat::{self, FdInfoUsage, Sensors, FdInfoStat, PerfCounter, PcieBw};

mod app;
use app::MyApp;
mod util;
use util::*;

const SPACE: f32 = 8.0;
const BASE: FontId = FontId::new(14.0, FontFamily::Monospace);
const MEDIUM: FontId = FontId::new(15.0, FontFamily::Monospace);
const HEADING: FontId = FontId::new(16.0, FontFamily::Monospace);
const HISTORY_LENGTH: Range<usize> = 0..30; // seconds

#[derive(Clone)]
pub struct CentralData {
    pub grbm: PerfCounter,
    pub grbm2: PerfCounter,
    pub grbm_history: Vec<History<u8>>,
    pub grbm2_history: Vec<History<u8>>,
    pub fdinfo: FdInfoStat,
    pub fdinfo_history: History<FdInfoUsage>,
    pub gpu_metrics: GpuMetrics,
    pub vram_usage: VramUsage,
    pub sensors: Sensors,
    pub sensors_history: SensorsHistory,
    pub pcie_bw_history: History<(u64, u64)>,
}

pub fn run(
    app_name: &str,
    title_with_version: &str,
    device_path: DevicePath,
    amdgpu_dev: DeviceHandle,
    device_path_list: &[DevicePath],
    interval: u64,
) {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let chip_class = ext_info.get_chip_class();
    let sysfs_path = pci_bus.get_sysfs_path();

    let mut grbm = PerfCounter::new_with_chip_class(stat::PCType::GRBM, chip_class);
    let mut grbm2 = PerfCounter::new_with_chip_class(stat::PCType::GRBM2, chip_class);

    let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
    let sample = Sampling::low();
    let mut fdinfo = FdInfoStat::new(sample.to_duration());
    {
        stat::update_index(&mut proc_index, &device_path);
        for pu in &proc_index {
            fdinfo.get_proc_usage(pu);
        }
    }

    let mut gpu_metrics = amdgpu_dev.get_gpu_metrics_from_sysfs_path(&sysfs_path)
        .unwrap_or(GpuMetrics::Unknown);
    let mut sensors = Sensors::new(&amdgpu_dev, &pci_bus);
    let mut vram_usage = VramUsage::new(&memory_info);
    let mut grbm_history = vec![History::new(HISTORY_LENGTH, f32::INFINITY); grbm.index.len()];
    let mut grbm2_history = vec![History::new(HISTORY_LENGTH, f32::INFINITY); grbm2.index.len()];
    let mut fdinfo_history = History::new(HISTORY_LENGTH, f32::INFINITY);
    let mut sensors_history = SensorsHistory::new();
    let pcie_bw = PcieBw::new(&sysfs_path);
    let share_pcie_bw = Arc::new(Mutex::new(pcie_bw.clone()));
    let mut pcie_bw_history: History<(u64, u64)> = History::new(HISTORY_LENGTH, f32::INFINITY);

    if pcie_bw.exists {
        let share_pcie_bw = share_pcie_bw.clone();
        let mut buf_pcie_bw = pcie_bw.clone();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(500)); // wait for user input

            buf_pcie_bw.update(); // msleep(1000)

            let lock = share_pcie_bw.lock();
            if let Ok(mut share_pcie_bw) = lock {
                *share_pcie_bw = buf_pcie_bw.clone();
            }
        });
    }

    let data = CentralData {
        grbm: grbm.clone(),
        grbm2: grbm2.clone(),
        grbm_history: grbm_history.clone(),
        grbm2_history: grbm2_history.clone(),
        vram_usage: vram_usage.clone(),
        fdinfo: fdinfo.clone(),
        fdinfo_history: fdinfo_history.clone(),
        gpu_metrics: gpu_metrics.clone(),
        sensors: sensors.clone(),
        sensors_history: sensors_history.clone(),
        pcie_bw_history: pcie_bw_history.clone(),
    };

    let app_device_info = AppDeviceInfo::new(&amdgpu_dev, &ext_info, &memory_info, &sensors);
    let device_list = device_path_list.iter().flat_map(DeviceListMenu::new).collect();
    let command_path = std::fs::read_link("/proc/self/exe").unwrap_or(PathBuf::from(app_name));

    let app = MyApp {
        app_device_info,
        device_list,
        command_path,
        hw_ip_info: libamdgpu_top::get_hw_ip_info_list(&amdgpu_dev),
        has_vcn_unified: libamdgpu_top::has_vcn_unified(&amdgpu_dev),
        support_pcie_bw: pcie_bw.exists,
        fdinfo_sort: Default::default(),
        reverse_sort: false,
        buf_data: data.clone(),
        arc_data: Arc::new(Mutex::new(data)),
        show_sidepanel: true,
    };

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1080.0, 840.0)),
        app_id: Some(app_name.to_string()),
        ..Default::default()
    };

    let share_proc_index = Arc::new(Mutex::new(proc_index));
    {
        let index = share_proc_index.clone();
        let mut buf_index: Vec<stat::ProcInfo> = Vec::new();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(interval));

            stat::update_index(&mut buf_index, &device_path);

            let lock = index.lock();
            if let Ok(mut index) = lock {
                *index = buf_index.clone();
            }
        });
    }

    {
        let now = std::time::Instant::now();
        let share_data = app.arc_data.clone();

        std::thread::spawn(move || loop {
            grbm.bits.clear();
            grbm2.bits.clear();

            for _ in 0..sample.count {
                grbm.read_reg(&amdgpu_dev);
                grbm2.read_reg(&amdgpu_dev);

                std::thread::sleep(sample.delay);
            }

            let sec = now.elapsed().as_secs_f64();

            for (pc, history) in [
                (&grbm, &mut grbm_history),
                (&grbm2, &mut grbm2_history),
            ] {
                for ((_name, pos), h) in pc.index.iter().zip(history.iter_mut()) {
                    h.add(sec, pc.bits.get(*pos));
                }
            }

            vram_usage.update_usage(&amdgpu_dev);
            sensors.update(&amdgpu_dev);
            sensors_history.add(sec, &sensors);

            if let Ok(v) = amdgpu_dev.get_gpu_metrics_from_sysfs_path(&sysfs_path) {
                gpu_metrics = v;
            }

            {
                let lock = share_pcie_bw.try_lock();
                if let Ok(pcie_bw) = lock {
                    let sent = pcie_bw.sent.saturating_mul(pcie_bw.max_payload_size as u64) >> 20;
                    let rec = pcie_bw.received.saturating_mul(pcie_bw.max_payload_size as u64) >> 20;

                    pcie_bw_history.add(sec, (sent, rec));
                }
            }

            {
                let lock = share_proc_index.lock();
                if let Ok(proc_index) = lock {
                    fdinfo.get_all_proc_usage(&proc_index);
                    fdinfo.interval = sample.to_duration();
                    fdinfo_history.add(sec, fdinfo.fold_fdinfo_usage());
                } else {
                    fdinfo.interval += sample.to_duration();
                }
            }

            {
                let lock = share_data.lock();
                if let Ok(mut share_data) = lock {
                    *share_data = CentralData {
                        grbm: grbm.clone(),
                        grbm2: grbm2.clone(),
                        grbm_history: grbm_history.clone(),
                        grbm2_history: grbm2_history.clone(),
                        vram_usage: vram_usage.clone(),
                        fdinfo: fdinfo.clone(),
                        fdinfo_history: fdinfo_history.clone(),
                        gpu_metrics: gpu_metrics.clone(),
                        sensors: sensors.clone(),
                        sensors_history: sensors_history.clone(),
                        pcie_bw_history: pcie_bw_history.clone(),
                    };
                }
            }
        });
    }

    eframe::run_native(
        title_with_version,
        options,
        Box::new(|_cc| Box::new(app)),
    ).unwrap();
}

impl MyApp {
    fn egui_side_panel(&self, ui: &mut egui::Ui) {
        ui.set_min_width(360.0);
        egui::ScrollArea::both().show(ui, |ui| {
            ui.add_space(SPACE);
            collapsing(ui, "Device Info", true, |ui| self.egui_app_device_info(ui));

            ui.add_space(SPACE);
            collapsing(ui, "Hardware IP Info", false, |ui| self.egui_hw_ip_info(ui));

            if self.app_device_info.decode.is_some() && self.app_device_info.encode.is_some() {
                ui.add_space(SPACE);
                collapsing(ui, "Video Caps Info", false, |ui| self.egui_video_caps_info(ui));
            }

            if self.app_device_info.vbios.is_some() {
                ui.add_space(SPACE);
                collapsing(ui, "VBIOS Info", false, |ui| self.egui_vbios_info(ui));
            }
            ui.add_space(SPACE);
        });
    }

    fn egui_central_panel(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(540.0);
        egui::ScrollArea::both().show(ui, |ui| {
            collapsing(ui, "GRBM", true, |ui| self.egui_perf_counter(
                ui,
                "GRBM",
                &self.buf_data.grbm,
                &self.buf_data.grbm_history,
            ));
            ui.add_space(SPACE);
            collapsing(ui, "GRBM2", true, |ui| self.egui_perf_counter(
                ui,
                "GRBM2",
                &self.buf_data.grbm2,
                &self.buf_data.grbm2_history,
            ));
            ui.add_space(SPACE);
            collapsing(ui, "VRAM", true, |ui| self.egui_vram(ui));
            ui.add_space(SPACE);
            collapsing(ui, "fdinfo", true, |ui| self.egui_grid_fdinfo(ui));
            ui.add_space(SPACE);
            collapsing(ui, "Sensors", true, |ui| self.egui_sensors(ui));

            if self.support_pcie_bw {
                ui.add_space(SPACE);
                collapsing(ui, "PCIe Bandwidth", true, |ui| self.egui_pcie_bw(ui));
            }

            let header = if let Some(h) = self.buf_data.gpu_metrics.get_header() {
                format!(
                    "GPU Metrics v{}.{}",
                    h.format_revision,
                    h.content_revision
                )
            } else {
                String::new()
            };

            match self.buf_data.gpu_metrics {
                GpuMetrics::V1_0(_) |
                GpuMetrics::V1_1(_) |
                GpuMetrics::V1_2(_) |
                GpuMetrics::V1_3(_) => {
                    ui.add_space(SPACE);
                    collapsing(ui, &header, true, |ui| self.egui_gpu_metrics_v1(ui));
                },
                GpuMetrics::V2_0(_) |
                GpuMetrics::V2_1(_) |
                GpuMetrics::V2_2(_) |
                GpuMetrics::V2_3(_) => {
                    ui.add_space(SPACE);
                    collapsing(ui, &header, true, |ui| self.egui_gpu_metrics_v2(ui));
                },
                GpuMetrics::Unknown => {},
            }
            ui.add_space(SPACE);
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        {
            let lock = self.arc_data.try_lock();
            if let Ok(data) = lock {
                self.buf_data = data.clone();
            }
        }
        {
            let mut style = (*ctx.style()).clone();
            style.override_font_id = Some(BASE);
            ctx.set_style(style);
        }
        ctx.clear_animations();

        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.show_sidepanel, "Info");
                self.egui_device_list(ui);
            });
        });

        if self.show_sidepanel {
            egui::SidePanel::left(egui::Id::new(3)).show(ctx, |ui| self.egui_side_panel(ui));
        }

        egui::CentralPanel::default().show(ctx, |ui| self.egui_central_panel(ui));

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
