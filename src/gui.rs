use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::PathBuf;
use eframe::egui;
use egui::{FontFamily, FontId, RichText};
use libdrm_amdgpu_sys::AMDGPU::{
    drm_amdgpu_info_device,
    drm_amdgpu_memory_info,
    DeviceHandle,
    GpuMetrics,
    MetricsInfo,
    CHIP_CLASS,
    GPU_INFO,
    SENSOR_INFO::*,
    VBIOS::VbiosInfo,
    VIDEO_CAPS::{VideoCapsInfo, CAP_TYPE},
};
use libdrm_amdgpu_sys::PCI;
use crate::{stat, DevicePath, Sampling};
use stat::{FdInfoSortType, FdInfoView, PerfCounter, VramUsageView};

const SPACE: f32 = 8.0;
const BASE: FontId = FontId::new(14.0, FontFamily::Monospace);
const MEDIUM: FontId = FontId::new(15.0, FontFamily::Monospace);
const HEADING: FontId = FontId::new(16.0, FontFamily::Monospace);

pub fn egui_run(instance: u32, update_process_index: u64, self_pid: i32) {
    let device_path = DevicePath::new(instance);
    let amdgpu_dev = device_path.init_device_handle();

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
    let sample = Sampling::low();
    let mut fdinfo = stat::FdInfoView::new(sample.to_duration());
    {
        stat::update_index(&mut proc_index, &device_path, self_pid);
        for pu in &proc_index {
            fdinfo.get_proc_usage(pu);
        }
    }

    let mut gpu_metrics = amdgpu_dev.get_gpu_metrics().unwrap_or(GpuMetrics::Unknown);
    let mut sensors = Sensors::new(&amdgpu_dev, &pci_bus);

    let data = CentralData {
        grbm: grbm.clone(),
        grbm2: grbm2.clone(),
        vram_usage: vram_usage.clone(),
        fdinfo: fdinfo.clone(),
        gpu_metrics: gpu_metrics.clone(),
        sensors: sensors.clone(),
    };

    let app = MyApp {
        info: MyApp::get_gpu_info(&amdgpu_dev, &ext_info, &memory_info),
        decode: amdgpu_dev.get_video_caps_info(CAP_TYPE::DECODE).ok(),
        encode: amdgpu_dev.get_video_caps_info(CAP_TYPE::ENCODE).ok(),
        vbios: amdgpu_dev.get_vbios_info().ok(),
        fdinfo_sort: FdInfoSortType::VRAM,
        reverse_sort: false,
        buf_data: data.clone(),
        arc_data: Arc::new(Mutex::new(data)),
    };

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1080.0, 840.0)),
        ..Default::default()
    };

    let share_proc_index = Arc::new(Mutex::new(proc_index));
    {
        let index = share_proc_index.clone();
        let mut buf_index: Vec<stat::ProcInfo> = Vec::new();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(update_process_index));

            stat::update_index(&mut buf_index, &device_path, self_pid);

            let lock = index.lock();
            if let Ok(mut index) = lock {
                *index = buf_index.clone();
            }
        });
    }

    {
        let share_data = app.arc_data.clone();

        std::thread::spawn(move || loop {
            grbm.bits.clear();
            grbm2.bits.clear();

            for _ in 0..sample.count {
                grbm.read_reg(&amdgpu_dev);
                grbm2.read_reg(&amdgpu_dev);

                std::thread::sleep(sample.delay);
            }

            vram_usage.update_usage(&amdgpu_dev);
            sensors.update(&amdgpu_dev);

            if let Ok(v) = amdgpu_dev.get_gpu_metrics() {
                gpu_metrics = v;
            }

            {
                let lock = share_proc_index.lock();
                if let Ok(proc_index) = lock {
                    fdinfo.proc_usage.clear();
                    fdinfo.drm_client_ids.clear();
                    for pu in proc_index.iter() {
                        fdinfo.get_proc_usage(pu);
                    }
                    fdinfo.interval = sample.to_duration();
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
                        vram_usage: vram_usage.clone(),
                        fdinfo: fdinfo.clone(),
                        gpu_metrics: gpu_metrics.clone(),
                        sensors: sensors.clone(),
                    };
                }
            }
        });
    }

    eframe::run_native(
        concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|_cc| Box::new(app)),
    ).unwrap();
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        {
            if let Ok(data) = self.arc_data.try_lock() {
                self.buf_data = data.clone();
            }
        }
        {
            let mut style = (*ctx.style()).clone();
            style.override_font_id = Some(BASE);
            ctx.set_style(style);
        }

        egui::SidePanel::left(egui::Id::new(3)).show(ctx, |ui| {
            ui.set_min_width(320.0);
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add_space(SPACE);
                egui::CollapsingHeader::new(
                    RichText::new("Device Info").font(HEADING)
                ).default_open(true).show(ui, |ui| self.egui_device_info(ui));

                if self.decode.is_some() && self.encode.is_some() {
                    ui.add_space(SPACE);
                    egui::CollapsingHeader::new(
                        RichText::new("Video Caps Info").font(HEADING)
                    ).default_open(false).show(ui, |ui| self.egui_video_caps_info(ui));
                }

                if self.vbios.is_some() {
                    ui.add_space(SPACE);
                    egui::CollapsingHeader::new(
                        RichText::new("VBIOS Info").font(HEADING)
                    ).default_open(false).show(ui, |ui| self.egui_vbios_info(ui));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_width(540.0);
            egui::ScrollArea::both().show(ui, |ui| {
                egui::CollapsingHeader::new(RichText::new("GRBM").font(HEADING))
                    .default_open(true)
                    .show(ui, |ui| self.egui_perf_counter(ui, "GRBM", &self.buf_data.grbm));
                ui.add_space(SPACE);
                egui::CollapsingHeader::new(RichText::new("GRBM2").font(HEADING))
                    .default_open(true)
                    .show(ui, |ui| self.egui_perf_counter(ui, "GRBM2", &self.buf_data.grbm2));
                ui.add_space(SPACE);
                egui::CollapsingHeader::new(RichText::new("VRAM").font(HEADING))
                    .default_open(true)
                    .show(ui, |ui| self.egui_vram(ui));
                ui.add_space(SPACE);
                egui::CollapsingHeader::new(RichText::new("fdinfo").font(HEADING))
                    .default_open(true)
                    .show(ui, |ui| self.egui_grid_fdinfo(ui));
                ui.add_space(SPACE);
                egui::CollapsingHeader::new(RichText::new("Sensors").font(HEADING))
                    .default_open(true)
                    .show(ui, |ui| self.egui_sensors(ui));

                let header = if let Some(h) = self.buf_data.gpu_metrics.get_header() {
                    format!(
                        "GPU Metrics v{}.{}",
                        h.format_revision,
                        h.content_revision
                    )
                } else {
                    "".to_string()
                };

                match self.buf_data.gpu_metrics {
                    GpuMetrics::V1_0(_) |
                    GpuMetrics::V1_1(_) |
                    GpuMetrics::V1_2(_) |
                    GpuMetrics::V1_3(_) => {
                        ui.add_space(SPACE);
                        egui::CollapsingHeader::new(
                            RichText::new(&header).font(HEADING)
                        )
                        .default_open(true)
                        .show(ui, |ui| self.egui_gpu_metrics_v1(ui, &self.buf_data.gpu_metrics));
                    },
                    GpuMetrics::V2_0(_) |
                    GpuMetrics::V2_1(_) |
                    GpuMetrics::V2_2(_) |
                    GpuMetrics::V2_3(_) => {
                        ui.add_space(SPACE);
                        egui::CollapsingHeader::new(
                            RichText::new(&header).font(HEADING)
                        )
                        .default_open(true)
                        .show(ui, |ui| self.egui_gpu_metrics_v2(ui, &self.buf_data.gpu_metrics));
                    },
                    GpuMetrics::Unknown => {},
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

#[derive(Clone)]
struct Sensors {
    hwmon_path: PathBuf,
    cur: PCI::LINK,
    max: PCI::LINK,
    bus_info: PCI::BUS_INFO,
    sclk: Option<u32>,
    mclk: Option<u32>,
    vddnb: Option<u32>,
    vddgfx: Option<u32>,
    temp: Option<u32>,
    critical_temp: u32,
    power: Option<u32>,
    power_cap: u32,
    fan_rpm: u32,
    fan_max_rpm: u32,
}

impl Sensors {
    pub fn new(amdgpu_dev: &DeviceHandle, pci_bus: &PCI::BUS_INFO) -> Self {
        let hwmon_path = pci_bus.get_hwmon_path().unwrap();
        let cur = pci_bus.get_link_info(PCI::STATUS::Current);
        let max = pci_bus.get_link_info(PCI::STATUS::Max);
        let [sclk, mclk, vddnb, vddgfx, temp, power] = [
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok(),
        ];
        let critical_temp = Self::parse_hwmon(hwmon_path.join("temp1_crit"))
            .saturating_div(1_000);
        let power_cap = Self::parse_hwmon(hwmon_path.join("power1_cap"))
            .saturating_div(1_000_000);
        let fan_rpm = Self::parse_hwmon(hwmon_path.join("fan1_input"));
        let fan_max_rpm = Self::parse_hwmon(hwmon_path.join("fan1_max"));

        Self {
            hwmon_path,
            cur,
            max,
            bus_info: *pci_bus,
            sclk,
            mclk,
            vddnb,
            vddgfx,
            temp,
            critical_temp,
            power,
            power_cap,
            fan_rpm,
            fan_max_rpm,
        }
    }

    fn parse_hwmon<P: Into<PathBuf>>(path: P) -> u32 {
        std::fs::read_to_string(path.into()).ok()
            .and_then(|file| file.trim_end().parse::<u32>().ok()).unwrap_or(0)
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.cur = self.bus_info.get_link_info(PCI::STATUS::Current);
        self.sclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok();
        self.mclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok();
        self.vddnb = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok();
        self.vddgfx = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok();
        self.temp = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP).ok();
        self.power = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok();
        self.fan_rpm = Self::parse_hwmon(self.hwmon_path.join("fan1_input"));
    }
}

#[derive(Clone)]
struct CentralData {
    grbm: PerfCounter,
    grbm2: PerfCounter,
    vram_usage: VramUsageView,
    fdinfo: FdInfoView,
    gpu_metrics: GpuMetrics,
    sensors: Sensors,
}

struct MyApp {
    info: Vec<(String, String)>,
    decode: Option<VideoCapsInfo>,
    encode: Option<VideoCapsInfo>,
    vbios: Option<VbiosInfo>,
    fdinfo_sort: FdInfoSortType,
    reverse_sort: bool,
    buf_data: CentralData,
    arc_data: Arc<Mutex<CentralData>>,
}

impl MyApp {
    pub fn get_gpu_info(
        amdgpu_dev: &DeviceHandle,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
    ) -> Vec<(String, String)> {
        let mut info: Vec<(String, String)> = Vec::new();

        let (min_gpu_clk, max_gpu_clk) =
            amdgpu_dev.get_min_max_gpu_clock().unwrap_or((0, 0));
        let (min_mem_clk, max_mem_clk) =
            amdgpu_dev.get_min_max_memory_clock().unwrap_or((0, 0));
        let re_bar = {
            let vram = memory_info.vram.total_heap_size;
            let cpu_accessible = memory_info.cpu_accessible_vram.total_heap_size;
            if (vram * 9 / 10) <= cpu_accessible {
                "Enabled"
            } else {
                "Disabled"
            }
        };
        let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
        let pad = ("".to_string(), "".to_string());
        if let Ok(mark_name) = amdgpu_dev.get_marketing_name() {
            info.push(("Device Name".to_string(), mark_name));
        }
        info.push((
            "DeviceID.RevID".to_string(),
            format!("{:#0X}.{:#0X}", ext_info.device_id(), ext_info.pci_rev_id()),
        ));
        info.push(pad.clone());
        info.push((
            "GPU Type".to_string(),
            if ext_info.is_apu() { "APU" } else { "dGPU" }.to_string(),
        ));
        info.push((
            "Family".to_string(),
            ext_info.get_family_name().to_string(),
        ));
        info.push((
            "ASIC Name".to_string(),
            ext_info.get_asic_name().to_string(),
        ));
        info.push((
            "Chip Class".to_string(),
            ext_info.get_chip_class().to_string(),
        ));
        info.push(pad.clone());

        let max_good_cu_per_sa = ext_info.get_max_good_cu_per_sa();
        let min_good_cu_per_sa = ext_info.get_min_good_cu_per_sa();

        info.push((
            "Shader Engine (SE)".to_string(),
            ext_info.max_se().to_string(),
        ));
        info.push((
            "Shader Array (SA/SH) per SE".to_string(),
            ext_info.max_sa_per_se().to_string(),
        ));
        if max_good_cu_per_sa != min_good_cu_per_sa {
            info.push((
                "CU per SA".to_string(),
                format!("[{min_good_cu_per_sa}, {max_good_cu_per_sa}]"),
            ));
        } else {
            info.push((
                "CU per SA".to_string(),
                max_good_cu_per_sa.to_string(),
            ));
        }
        info.push((
            "Total CU".to_string(),
            ext_info.cu_active_number().to_string(),
        ));

        let rb_pipes = ext_info.rb_pipes();
        let rop_count = ext_info.calc_rop_count();

        if ext_info.get_asic_name().rbplus_allowed() {
            info.push((
                "RenderBackendPlus (RB+)".to_string(),
                format!("{rb_pipes} ({rop_count} ROPs)"),
            ));
        } else {
            info.push((
                "RenderBackend (RB)".to_string(),
                format!("{rb_pipes} ({rop_count} ROPs)"),
            ));
        }

        info.push((
            "Peak Pixel Fill-Rate".to_string(),
            format!("{} GP/s", rop_count * max_gpu_clk / 1000),
        ));

        info.push((
            "GPU Clock".to_string(),
            format!("{min_gpu_clk}-{max_gpu_clk} MHz"),
        ));
        info.push((
            "Peak FP32".to_string(),
            format!("{} GFLOPS", ext_info.peak_gflops()),
        ));

        info.push(pad.clone());
        info.push((
            "VRAM Type".to_string(),
            ext_info.get_vram_type().to_string(),
        ));
        info.push((
            "VRAM Bit Width".to_string(),
            format!("{}-bit", ext_info.vram_bit_width),
        ));
        info.push((
            "VRAM Size".to_string(),
            format!("{} MiB", memory_info.vram.total_heap_size >> 20),
        ));
        info.push((
            "Memory Clock".to_string(),
            format!("{min_mem_clk}-{max_mem_clk} MHz"),
        ));
        info.push((
            "ResizableBAR".to_string(),
            re_bar.to_string(),
        ));
        info.push(pad.clone());

        let link = pci_bus.get_link_info(PCI::STATUS::Max);
        info.push((
            "PCI (domain:bus:dev.func)".to_string(),
            pci_bus.to_string(),
        ));
        info.push((
            "PCI Link Speed (Max)".to_string(),
            format!("Gen{}x{}", link.gen, link.width),
        ));

        info
    }

    fn egui_device_info(&self, ui: &mut egui::Ui) {
        egui::Grid::new("device_info").show(ui, |ui| {
            for (label, val) in &self.info {
                ui.label(label);
                ui.label(val);
                ui.end_row();
            }
        });
    }

    fn egui_video_caps_info(&self, ui: &mut egui::Ui) {
        let Some(ref decode_caps) = self.decode else { return };
        let Some(ref encode_caps) = self.encode else { return };

        egui::Grid::new("codec_info").show(ui, |ui| {
            ui.label("Codec").highlight();
            ui.label("Decode").highlight();
            ui.label("Encode").highlight();
            ui.end_row();
            
            for (name, decode, encode) in [
                ("MPEG2", decode_caps.mpeg2, encode_caps.mpeg2),
                ("MPEG4", decode_caps.mpeg4, encode_caps.mpeg4),
                ("VC1", decode_caps.vc1, encode_caps.vc1),
                ("MPEG4_AVC", decode_caps.mpeg4_avc, encode_caps.mpeg4_avc),
                ("HEVC", decode_caps.hevc, encode_caps.hevc),
                ("JPEG", decode_caps.jpeg, encode_caps.jpeg),
                ("VP9", decode_caps.vp9, encode_caps.vp9),
                ("AV1", decode_caps.av1, encode_caps.av1),
            ] {
                ui.label(name).highlight();
                if let Some(dec) = decode {
                    ui.label(&format!("{}x{}", dec.max_width, dec.max_height));
                } else {
                    ui.label("N/A");
                }
                if let Some(enc) = encode {
                    ui.label(&format!("{}x{}", enc.max_width, enc.max_height));
                } else {
                    ui.label("N/A");
                }
                ui.end_row();
            }
        });
    }

    fn egui_vbios_info(&self, ui: &mut egui::Ui) {
        let Some(ref vbios) = self.vbios else { return };
        egui::Grid::new("vbios_info").show(ui, |ui| {
            for (name, val) in [
                ("Name", &vbios.name),
                ("PN", &vbios.pn),
                ("Version", &vbios.ver),
                ("Date", &vbios.date),
            ] {
                ui.label(name).highlight();
                ui.label(val);
                ui.end_row();
            }
        });
    }

    fn egui_perf_counter(&self, ui: &mut egui::Ui, name: &str, pc: &PerfCounter) {
        egui::Grid::new(name).show(ui, |ui| {
            for (name, pos) in &pc.index {
                let usage = pc.bits.get(*pos);
                ui.label(name);
                ui.add_sized([240.0, 0.0], egui::ProgressBar::new(
                    (usage as f32) / 100.0
                ).text(RichText::new(format!("{usage:3} %")).font(BASE)));
                ui.end_row();
            }
        });
    }

    fn egui_vram(&self, ui: &mut egui::Ui) {
        egui::Grid::new("VRAM").show(ui, |ui| {
            for (v, name) in [
                (&self.buf_data.vram_usage.vram, "VRAM"),
                (&self.buf_data.vram_usage.gtt, "GTT"),
            ] {
                let progress = (v.usage >> 20) as f32 / (v.total >> 20) as f32;
                let text = format!(
                    "{:5} / {:5} MiB",
                    v.usage >> 20,
                    v.total >> 20,
                );
                let bar = egui::ProgressBar::new(progress)
                    .text(RichText::new(&text).font(BASE));
                ui.label(name);
                ui.add_sized([360.0, 16.0], bar);
                ui.end_row();
            }
        });
    }

    fn egui_grid_fdinfo(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("fdinfo").show(ui, |ui| {
            {
                ui.style_mut().override_font_id = Some(MEDIUM);
            }
            ui.label(rt_base(format!("{:^15}", "Name"))).highlight();
            ui.label(rt_base(format!("{:^8}", "PID"))).highlight();
            if ui.button(rt_base(format!("{:^10}", "VRAM"))).clicked() {
                if let FdInfoSortType::VRAM = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::VRAM;
            };
            if ui.button(rt_base(format!("{:^5}", "GFX"))).clicked() {
                if let FdInfoSortType::GFX = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::GFX;
            };
            ui.label(rt_base("Compute")).highlight();
            ui.label(rt_base(format!("{:^5}", "DMA"))).highlight();
            if ui.button(rt_base("Decode")).clicked() {
                if let FdInfoSortType::Decode = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::Decode;
            };
            if ui.button(rt_base("Encode")).clicked() {
                if let FdInfoSortType::Encode = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::Encode;
            };
            ui.end_row();

            stat::sort_proc_usage(
                &mut self.buf_data.fdinfo.proc_usage,
                &self.fdinfo_sort,
                self.reverse_sort,
            );

            for pu in &self.buf_data.fdinfo.proc_usage {
                ui.label(pu.name.to_string());
                ui.label(format!("{:>8}", pu.pid));
                ui.label(&format!("{:5} MiB", pu.usage.vram_usage >> 10));
                let dec_usage = pu.usage.dec + pu.usage.vcn_jpeg;
                let enc_usage = pu.usage.enc + pu.usage.uvd_enc;
                for usage in [
                    pu.usage.gfx,
                    pu.usage.compute,
                    pu.usage.dma,
                    dec_usage,
                    enc_usage,
                ] {
                    ui.label(&format!("{usage:3} %"));
                }
                ui.end_row();
            } // proc_usage
        });
    }

    fn egui_sensors(&self, ui: &mut egui::Ui) {
        let sensors = &self.buf_data.sensors;
        egui::Grid::new("Sensors").show(ui, |ui| {
            {
                ui.style_mut().override_font_id = Some(MEDIUM);
            }
            let mut c = 0;
            for (name, val, unit) in [
                ("GFX_SCLK", &sensors.sclk, "MHz"),
                ("GFX_MCLK", &sensors.mclk, "MHz"),
                ("VDDNB", &sensors.vddnb, "mV"),
                ("VDDGFX", &sensors.vddgfx, "mV"),
            ] {
                let Some(val) = val else { continue };
                ui.label(name);
                ui.label("=>");
                ui.label(&format!("{val:5} {unit}"));
                c += 1;
                if c % 2 == 0 { ui.end_row(); }
            }
        });
        if let Some(temp) = &sensors.temp {
            ui.label(&format!(
                "GPU Temp. => {temp:3} C (Crit. {critical} C)",
                temp = temp.saturating_div(1_000),
                critical = sensors.critical_temp,
            ));
        }
        if let Some(power) = &sensors.power {
            ui.label(&format!(
                "GPU Power => {power:3} C (Cap. {cap} W)",
                cap = sensors.power_cap,
            ));
        }
        {
            ui.label(&format!(
                "Fan => {fan:4} RPM (Max. {max} RPM)",
                fan = sensors.fan_rpm,
                max = sensors.fan_max_rpm,
            ));
        }
        ui.label(&format!(
            "PCI Link Speed => Gen{cur_gen}x{cur_width:<2} (Max. Gen{max_gen}x{max_width})",
            cur_gen = sensors.cur.gen,
            cur_width = sensors.cur.width,
            max_gen = sensors.max.gen,
            max_width = sensors.max.width,
        ));
    }

    fn egui_gpu_metrics_v1(&self, ui: &mut egui::Ui, gpu_metrics: &GpuMetrics) {
        if let Some(socket_power) = gpu_metrics.get_average_socket_power() {
            if socket_power != u16::MAX {
                ui.label(&format!("Socket Power => {socket_power:3} W"));
            }
        }

        ui.horizontal(|ui| {
            for (val, name) in [
                (gpu_metrics.get_temperature_edge(), "Edge"),
                (gpu_metrics.get_temperature_hotspot(), "Hotspot"),
                (gpu_metrics.get_temperature_mem(), "Memory"),
            ] {
                let Some(v) = val.and_then(|v| v.ne(&u16::MAX).then_some(v)) else { continue };
                ui.label(&format!("{name} => {v:3} C,"));
            }
        });

        ui.horizontal(|ui| {
            for (val, name) in [
                (gpu_metrics.get_temperature_vrgfx(), "VRGFX"),
                (gpu_metrics.get_temperature_vrsoc(), "VRSOC"),
                (gpu_metrics.get_temperature_vrmem(), "VRMEM"),
            ] {
                let Some(v) = val.and_then(|v| v.ne(&u16::MAX).then_some(v)) else { continue };
                ui.label(&format!("{name} => {v:3} C,"));
            }
        });

        ui.horizontal(|ui| {
            for (val, name) in [
                (gpu_metrics.get_voltage_soc(), "SoC"),
                (gpu_metrics.get_voltage_gfx(), "GFX"),
                (gpu_metrics.get_voltage_mem(), "Mem"),
            ] {
                let Some(v) = val.and_then(|v| v.ne(&u16::MAX).then_some(v)) else { continue };
                ui.label(&format!("{name} => {v:4} mV,"));
            }
        });
    }

    fn egui_gpu_metrics_v2(&self, ui: &mut egui::Ui, gpu_metrics: &GpuMetrics) {
        const CORE_TEMP_LABEL: &str = "Core Temp (C)";
        const CORE_POWER_LABEL: &str = "Core Power (mW)";
        const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
        const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
        const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

        ui.horizontal(|ui| {
            ui.label("GFX =>");
            for (val, unit, div) in [
                (gpu_metrics.get_temperature_gfx(), "C", 100),
                (gpu_metrics.get_average_gfx_power(), "mW", 1),
                (gpu_metrics.get_current_gfxclk(), "MHz", 1),
            ] {
                let v = val
                    .and_then(|v| v.ne(&u16::MAX).then_some(v))
                    .unwrap_or(0)
                    .saturating_div(div);
                ui.label(&format!("{v:5} {unit}"));
            }
        });

        ui.horizontal(|ui| {
            ui.label("SoC =>");
            for (val, unit, div) in [
                (gpu_metrics.get_temperature_soc(), "C", 100),
                (gpu_metrics.get_average_soc_power(), "mW", 1),
                (gpu_metrics.get_current_socclk(), "MHz", 1),
            ] {
                let v = val
                    .and_then(|v| v.ne(&u16::MAX).then_some(v))
                    .unwrap_or(0)
                    .saturating_div(div);
                ui.label(&format!("{v:5} {unit}"));
            }
        });

        if let Some(socket_power) = gpu_metrics.get_average_socket_power() {
            if socket_power != u16::MAX {
                ui.label(&format!("Socket Power => {socket_power:3} W"));
            }
        }

        let for_array = |v: &u16, div: u16, ui: &mut egui::Ui| {
            let v = if v == &u16::MAX {
                0
            } else {
                v.saturating_div(div)
            };

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::TOP),
                |ui| ui.label(&format!("{v:5},")),
            );
        };

        egui::Grid::new("GPU Metrics v2.x Core/L3").show(ui, |ui| {
            for (val, label, div) in [
                (gpu_metrics.get_temperature_core(), CORE_TEMP_LABEL, 100),
                (gpu_metrics.get_average_core_power(), CORE_POWER_LABEL, 1),
                (gpu_metrics.get_current_coreclk(), CORE_CLOCK_LABEL, 1),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label("=> [");
                for v in &val {
                    for_array(v, div, ui);
                }
                ui.label("]");
                ui.end_row();
            }

            for (val, label, div) in [
                (gpu_metrics.get_temperature_l3(), L3_TEMP_LABEL, 100),
                (gpu_metrics.get_current_l3clk(), L3_CLOCK_LABEL, 1),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label("=> [");
                for v in &val {
                    for_array(v, div, ui);
                }
                ui.label("]");
                ui.end_row();
            }
        });
    }
}

fn rt_base<T: Into<String>>(s: T) -> RichText {
    RichText::new(s.into()).font(BASE)
}
