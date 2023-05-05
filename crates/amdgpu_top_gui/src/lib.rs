use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::PathBuf;
use eframe::egui;
use egui::{FontFamily, FontId, RichText, util::History};
use egui::plot::{Corner, Legend, Line, Plot, PlotPoint, PlotPoints};
use std::ops::{Range, RangeInclusive};

use libamdgpu_top::AMDGPU::{
    drm_amdgpu_info_device,
    drm_amdgpu_memory_info,
    DeviceHandle,
    GpuMetrics,
    MetricsInfo,
    CHIP_CLASS,
    GPU_INFO,
    HW_IP::{HwIpInfo, HW_IP_TYPE},
    VBIOS::VbiosInfo,
    VIDEO_CAPS::{VideoCapsInfo, CAP_TYPE},
};
use libamdgpu_top::PCI;
use libamdgpu_top::{stat, DevicePath, Sampling, VramUsage};
use stat::{check_metrics_val, FdInfoUsage, Sensors, FdInfoSortType, FdInfoStat, PerfCounter};

const SPACE: f32 = 8.0;
const BASE: FontId = FontId::new(14.0, FontFamily::Monospace);
const MEDIUM: FontId = FontId::new(15.0, FontFamily::Monospace);
const HEADING: FontId = FontId::new(16.0, FontFamily::Monospace);
const HISTORY_LENGTH: Range<usize> = 0..30; // seconds
const PLOT_HEIGHT: f32 = 32.0;
const PLOT_WIDTH: f32 = 240.0;

const HW_IP_LIST: &[HW_IP_TYPE] = &[
    HW_IP_TYPE::GFX,
    HW_IP_TYPE::COMPUTE,
    HW_IP_TYPE::DMA,
    HW_IP_TYPE::UVD,
    HW_IP_TYPE::VCE,
    HW_IP_TYPE::UVD_ENC,
    HW_IP_TYPE::VCN_DEC,
    HW_IP_TYPE::VCN_ENC,
    HW_IP_TYPE::VCN_JPEG,
];

struct DeviceListMenu {
    instance: u32,
    name: String,
    pci: PCI::BUS_INFO,
}

impl DeviceListMenu {
    fn new(device_path: &DevicePath) -> Option<Self> {
        let instance = device_path.get_instance_number()?;
        let pci = device_path.pci?;
        let name = {
            let amdgpu_dev = device_path.init().ok()?;
            amdgpu_dev.get_marketing_name().unwrap_or_default()
        };

        Some(Self {
            instance,
            pci,
            name,
        })
    }
}

#[derive(Debug, Clone)]
struct SensorsHistory {
    sclk: History<u32>,
    mclk: History<u32>,
    vddgfx: History<u32>,
    vddnb: History<u32>,
    temp: History<u32>,
    power: History<u32>,
    fan_rpm: History<u32>,
}

impl SensorsHistory {
    fn new() -> Self {
        let [sclk, mclk, vddgfx, vddnb, temp, power, fan_rpm] = [0; 7]
            .map(|_| History::new(HISTORY_LENGTH, f32::INFINITY));

        Self { sclk, mclk, vddgfx, vddnb, temp, power, fan_rpm }
    }

    fn add(&mut self, sec: f64, sensors: &Sensors) {
        for (history, val) in [
            (&mut self.sclk, sensors.sclk),
            (&mut self.mclk, sensors.mclk),
            (&mut self.vddgfx, sensors.vddgfx),
            (&mut self.vddnb, sensors.vddnb),
            (&mut self.temp, sensors.temp.map(|v| v.saturating_div(1000))),
            (&mut self.power, sensors.power),
            (&mut self.fan_rpm, sensors.fan_rpm),
        ] {
            let Some(val) = val else { continue };
            history.add(sec, val);
        }
    }
}

pub fn run(
    title: &str,
    device_path: DevicePath,
    amdgpu_dev: DeviceHandle,
    device_path_list: &[DevicePath],
    interval: u64,
) {
    let self_pid = 0; // no filtering in GUI
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let chip_class = ext_info.get_chip_class();

    let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
        stat::GFX10_GRBM_INDEX
    } else {
        stat::GRBM_INDEX
    };

    let mut grbm = PerfCounter::new(stat::PCType::GRBM, grbm_index);
    let mut grbm2 = PerfCounter::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);

    let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
    let sample = Sampling::low();
    let mut fdinfo = FdInfoStat::new(sample.to_duration());
    {
        stat::update_index(&mut proc_index, &device_path, self_pid);
        for pu in &proc_index {
            fdinfo.get_proc_usage(pu);
        }
    }

    let mut gpu_metrics = amdgpu_dev.get_gpu_metrics().unwrap_or(GpuMetrics::Unknown);
    let mut sensors = Sensors::new(&amdgpu_dev, &pci_bus);
    let mut vram_usage = VramUsage::new(&memory_info);
    let mut grbm_history = vec![History::new(HISTORY_LENGTH, f32::INFINITY); grbm.index.len()];
    let mut grbm2_history = vec![History::new(HISTORY_LENGTH, f32::INFINITY); grbm2.index.len()];
    let mut fdinfo_history = History::new(HISTORY_LENGTH, f32::INFINITY);
    let mut sensors_history = SensorsHistory::new();

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
    };

    let app_device_info = AppDeviceInfo::new(&amdgpu_dev, &ext_info, &memory_info, &sensors);
    let device_list = device_path_list.iter().flat_map(DeviceListMenu::new).collect();
    let command_path = std::fs::read_link("/proc/self/exe")
        .unwrap_or(PathBuf::from("amdgpu_top"));

    let app = MyApp {
        app_device_info,
        device_list,
        command_path,
        decode: amdgpu_dev.get_video_caps_info(CAP_TYPE::DECODE).ok(),
        encode: amdgpu_dev.get_video_caps_info(CAP_TYPE::ENCODE).ok(),
        vbios: amdgpu_dev.get_vbios_info().ok(),
        fdinfo_sort: Default::default(),
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
            std::thread::sleep(Duration::from_secs(interval));

            stat::update_index(&mut buf_index, &device_path, self_pid);

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

            if let Ok(v) = amdgpu_dev.get_gpu_metrics() {
                gpu_metrics = v;
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
                    };
                }
            }
        });
    }

    eframe::run_native(
        title,
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
        ctx.clear_animations();

        egui::SidePanel::left(egui::Id::new(3)).show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.add_space(SPACE / 2.0);
            self.egui_device_list(ui);
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add_space(SPACE);
                collapsing(ui, "Device Info", true, |ui| self.egui_app_device_info(ui));

                ui.add_space(SPACE);
                collapsing(ui, "Hardware IP Info", false, |ui| self.egui_hw_ip_info(ui));

                if self.decode.is_some() && self.encode.is_some() {
                    ui.add_space(SPACE);
                    collapsing(ui, "Video Caps Info", false, |ui| self.egui_video_caps_info(ui));
                }

                if self.vbios.is_some() {
                    ui.add_space(SPACE);
                    collapsing(ui, "VBIOS Info", false, |ui| self.egui_vbios_info(ui));
                }
                ui.add_space(SPACE);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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
        });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

#[derive(Clone)]
struct CentralData {
    grbm: PerfCounter,
    grbm2: PerfCounter,
    grbm_history: Vec<History<u8>>,
    grbm2_history: Vec<History<u8>>,
    fdinfo: FdInfoStat,
    fdinfo_history: History<FdInfoUsage>,
    gpu_metrics: GpuMetrics,
    vram_usage: VramUsage,
    sensors: Sensors,
    sensors_history: SensorsHistory,
}

struct MyApp {
    command_path: PathBuf,
    app_device_info: AppDeviceInfo,
    device_list: Vec<DeviceListMenu>,
    decode: Option<VideoCapsInfo>,
    encode: Option<VideoCapsInfo>,
    vbios: Option<VbiosInfo>,
    fdinfo_sort: FdInfoSortType,
    reverse_sort: bool,
    buf_data: CentralData,
    arc_data: Arc<Mutex<CentralData>>,
}

#[derive(Clone)]
struct AppDeviceInfo {
    ext_info: drm_amdgpu_info_device,
    memory_info: drm_amdgpu_memory_info,
    hw_ip_info: Vec<HwIpInfo>,
    resizable_bar: bool,
    min_gpu_clk: u32,
    max_gpu_clk: u32,
    min_mem_clk: u32,
    max_mem_clk: u32,
    marketing_name: String,
    pci_bus: PCI::BUS_INFO,
    critical_temp: Option<u32>,
    power_cap: Option<u32>,
    fan_max_rpm: Option<u32>,
}

impl AppDeviceInfo {
    fn new(
        amdgpu_dev: &DeviceHandle,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
        sensors: &Sensors,
    ) -> Self {
        let (min_gpu_clk, max_gpu_clk) =
            amdgpu_dev.get_min_max_gpu_clock().unwrap_or((0, 0));
        let (min_mem_clk, max_mem_clk) =
            amdgpu_dev.get_min_max_memory_clock().unwrap_or((0, 0));
        let resizable_bar = memory_info.check_resizable_bar();
        let marketing_name = amdgpu_dev.get_marketing_name().unwrap_or_default();
        let hw_ip_info = HW_IP_LIST.iter()
            .filter_map(|ip_type| amdgpu_dev.get_hw_ip_info(*ip_type).ok())
            .filter(|hw_ip_info| hw_ip_info.count != 0).collect();

        Self {
            ext_info: *ext_info,
            memory_info: *memory_info,
            hw_ip_info,
            resizable_bar,
            min_gpu_clk,
            max_gpu_clk,
            min_mem_clk,
            max_mem_clk,
            marketing_name,
            pci_bus: sensors.bus_info,
            critical_temp: sensors.critical_temp,
            power_cap: sensors.power_cap,
            fan_max_rpm: sensors.fan_max_rpm,
        }
    }
}

impl MyApp {
    pub fn egui_device_list(&self, ui: &mut egui::Ui) {
        ui.menu_button(RichText::new("Device List").font(BASE), |ui| {
            ui.set_width(360.0);
            for device in &self.device_list {
                ui.horizontal(|ui| {
                    let text = RichText::new(format!(
                        "#{instance} {name} ({pci})",
                        instance = device.instance,
                        name = device.name,
                        pci = device.pci,
                    )).font(BASE);

                    if self.app_device_info.pci_bus == device.pci {
                        ui.add_enabled(false, egui::Button::new(text));
                    } else {
                        ui.menu_button(text, |ui| {
                            if ui.button("Launch in a new process").clicked() {
                                std::process::Command::new(&self.command_path)
                                    .args(["--gui", "--pci", &device.pci.to_string()])
                                    .spawn()
                                    .unwrap();
                            }
                        });
                    }
                });
            }
        });
    }

    pub fn egui_app_device_info(&self, ui: &mut egui::Ui) {
        egui::Grid::new("app_device_info").show(ui, |ui| {
            let ext_info = &self.app_device_info.ext_info;
            let memory_info = &self.app_device_info.memory_info;
            let pci_bus = &self.app_device_info.pci_bus;
            let (min_gpu_clk, max_gpu_clk) = (
                &self.app_device_info.min_gpu_clk,
                &self.app_device_info.max_gpu_clk,
            );
            let (min_mem_clk, max_mem_clk) = (
                &self.app_device_info.min_mem_clk,
                &self.app_device_info.max_mem_clk,
            );

            let dev_id = format!("{:#0X}.{:#0X}", ext_info.device_id(), ext_info.pci_rev_id());
            let gpu_type = if ext_info.is_apu() { "APU" } else { "dGPU" };
            let family = ext_info.get_family_name();
            let asic = ext_info.get_asic_name();
            let chip_class = ext_info.get_chip_class();

            let grid = |ui: &mut egui::Ui, v: &[(&str, &str)]| {
                for (name, val) in v {
                    ui.label(*name);
                    ui.label(*val);
                    ui.end_row();
                }
            };

            grid(ui, &[
                ("Device Name", &self.app_device_info.marketing_name),
                ("DeviceID.RevID", &dev_id),
                ("GPU Type", gpu_type),
                ("Family", &family.to_string()),
                ("ASIC Name", &asic.to_string()),
                ("Chip Class", &chip_class.to_string()),
            ]);
            ui.end_row();

            let max_good_cu_per_sa = ext_info.get_max_good_cu_per_sa();
            let min_good_cu_per_sa = ext_info.get_min_good_cu_per_sa();
            let cu_per_sa = if max_good_cu_per_sa != min_good_cu_per_sa {
                format!("[{min_good_cu_per_sa}, {max_good_cu_per_sa}]")
            } else {
                max_good_cu_per_sa.to_string()
            };
            let rb_pipes = ext_info.rb_pipes();
            let rop_count = ext_info.calc_rop_count();
            let rb_type = if asic.rbplus_allowed() {
                "RenderBackendPlus (RB+)"
            } else {
                "RenderBackend (RB)"
            };
            let peak_gp = format!("{} GP/s", rop_count * max_gpu_clk / 1000);
            let peak_fp32 = format!("{} GFLOPS", ext_info.peak_gflops());

            grid(ui, &[
                ("Shader Engine (SE)", &ext_info.max_se().to_string()),
                ("Shader Array (SA/SH) per SE", &ext_info.max_sa_per_se().to_string()),
                ("CU per SA", &cu_per_sa),
                ("Total CU", &ext_info.cu_active_number().to_string()),
                (rb_type, &format!("{rb_pipes} ({rop_count} ROPs)")),
                ("Peak Pixel Fill-Rate", &peak_gp),
                ("GPU Clock", &format!("{min_gpu_clk}-{max_gpu_clk} MHz")),
                ("Peak FP32", &peak_fp32),
            ]);
            ui.end_row();

            let re_bar = if self.app_device_info.resizable_bar {
                "Enabled"
            } else {
                "Disabled"
            };

            grid(ui, &[
                ("VRAM Type", &ext_info.get_vram_type().to_string()),
                ("VRAM Bit Width", &format!("{}-bit", ext_info.vram_bit_width)),
                ("VRAM Size", &format!("{} MiB", memory_info.vram.total_heap_size >> 20)),
                ("Memory Clock", &format!("{min_mem_clk}-{max_mem_clk} MHz")),
                ("ResizableBAR", re_bar),
            ]);
            ui.end_row();

            let gl1_cache_size = ext_info.get_gl1_cache_size();
            let l3_cache_size = ext_info.calc_l3_cache_size_mb();

            ui.label("L1 Cache (per CU)");
            ui.label(format!("{:4} KiB", ext_info.get_l1_cache_size() / 1024));
            ui.end_row();
            if 0 < gl1_cache_size {
                ui.label("GL1 Cache (per SA/SH)");
                ui.label(format!("{gl1_cache_size:4} KiB"));
                ui.end_row();
            }
            ui.label("L2 Cache");
            ui.label(format!(
                "{:4} KiB ({} Banks)",
                ext_info.calc_l2_cache_size() / 1024,
                ext_info.num_tcc_blocks
            ));
            ui.end_row();
            if 0 < l3_cache_size {
                ui.label("L3 Cache (MALL)");
                ui.label(format!("{l3_cache_size:4} MiB"));
                ui.end_row();
            }
            ui.end_row();

            let link = pci_bus.get_link_info(PCI::STATUS::Max);

            grid(ui, &[
                ("PCI (domain:bus:dev.func)", &pci_bus.to_string()),
                ("PCI Link Speed (Max)", &format!("Gen{}x{}", link.gen, link.width)),
            ]);
            ui.end_row();

            for (label, val, unit) in [
                ("Critical Temp.", &self.app_device_info.critical_temp, "C"),
                ("Power Cap.", &self.app_device_info.power_cap, "W"),
                ("Fan RPM (Max).", &self.app_device_info.fan_max_rpm, "RPM"),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label(format!("{val:4} {unit}"));
                ui.end_row();
            }
        });
    }

    pub fn egui_hw_ip_info(&self, ui: &mut egui::Ui) {
        egui::Grid::new("hw_ip_info").show(ui, |ui| {
            ui.label("IP").highlight();
            ui.label("version").highlight();
            ui.label("queues").highlight();
            ui.end_row();

            for hw_ip in &self.app_device_info.hw_ip_info {
                let (major, minor) = hw_ip.info.version();

                ui.label(hw_ip.ip_type.to_string());
                ui.label(format!("{major}.{minor}"));
                ui.label(hw_ip.info.num_queues().to_string());
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
                ui.label(name);
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

    fn egui_perf_counter(
        &self,
        ui: &mut egui::Ui,
        name: &str,
        pc: &PerfCounter,
        history: &[History<u8>],
    ) {
        let y_fmt = |_y: f64, _range: &RangeInclusive<f64>| {
            String::new()
        };
        let label_fmt = |_s: &str, val: &PlotPoint| {
            format!("{:.1}s : {:.0}%", val.x, val.y)
        };

        egui::Grid::new(name).show(ui, |ui| {
            for ((name, pos), history) in pc.index.iter().zip(history.iter()) {
                let usage = pc.bits.get(*pos);
                ui.label(name);
                ui.label(format!("{usage:3}%"));

                let points: PlotPoints = history.iter()
                    .map(|(i, val)| [i, val as f64]).collect();
                let line = Line::new(points).fill(1.0);
                Plot::new(name)
                    .allow_drag(false)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .show_axes([false, true])
                    .include_y(0.0)
                    .include_y(100.0)
                    .y_axis_formatter(y_fmt)
                    .label_formatter(label_fmt)
                    .auto_bounds_x()
                    .height(PLOT_HEIGHT)
                    .width(PLOT_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
                ui.end_row();
            }
        });
    }

    fn egui_vram(&self, ui: &mut egui::Ui) {
        egui::Grid::new("VRAM").show(ui, |ui| {
            for (v, name) in [
                (&self.buf_data.vram_usage.0.vram, "VRAM"),
                (&self.buf_data.vram_usage.0.cpu_accessible_vram, "CPU-Visible VRAM"),
                (&self.buf_data.vram_usage.0.gtt, "GTT"),
            ] {
                let progress = (v.heap_usage >> 20) as f32 / (v.total_heap_size >> 20) as f32;
                let text = format!("{:5} / {:5} MiB", v.heap_usage >> 20, v.total_heap_size >> 20);
                let bar = egui::ProgressBar::new(progress)
                    .text(RichText::new(&text).font(BASE));
                ui.label(RichText::new(name).font(MEDIUM));
                ui.add_sized([360.0, 16.0], bar);
                ui.end_row();
            }
        });
    }

    fn set_fdinfo_sort_type(&mut self, sort_type: FdInfoSortType) {
        if sort_type == self.fdinfo_sort {
            self.reverse_sort ^= true;
        } else {
            self.reverse_sort = false;
        }
        self.fdinfo_sort = sort_type;
    }

    fn egui_fdinfo_plot(&self, ui: &mut egui::Ui) {
        let y_fmt = |_y: f64, _range: &RangeInclusive<f64>| {
            String::new()
        };
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0}%", val.x, val.y)
        };

        let [mut gfx, mut compute, mut dma, mut dec, mut enc] = [0; 5]
            .map(|_| Vec::<[f64; 2]>::with_capacity(HISTORY_LENGTH.end));

        for (i, usage) in self.buf_data.fdinfo_history.iter() {
            let usage_dec = usage.dec + usage.vcn_jpeg;
            let usage_enc = usage.enc + usage.uvd_enc;

            gfx.push([i, usage.gfx as f64]);
            compute.push([i, usage.compute as f64]);
            dma.push([i, usage.dma as f64]);
            dec.push([i, usage_dec as f64]);
            enc.push([i, usage_enc as f64]);
        }

        Plot::new("fdinfo plot")
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .show_axes([false, true])
            .include_y(0.0)
            .include_y(100.0)
            .y_axis_formatter(y_fmt)
            .label_formatter(label_fmt)
            .auto_bounds_x()
            .height(ui.available_width() / 4.0)
            .width(ui.available_width() - 24.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (usage, name) in [
                    (gfx, "GFX"),
                    (compute, "Compute"),
                    (dma, "DMA"),
                    (dec, "Decode"),
                    (enc, "Encode"),
                ] {
                    plot_ui.line(Line::new(PlotPoints::new(usage)).name(name));
                }
            });
    }

    fn egui_grid_fdinfo(&mut self, ui: &mut egui::Ui) {
        collapsing_plot(ui, "fdinfo Plot", true, |ui| self.egui_fdinfo_plot(ui));

        egui::Grid::new("fdinfo").show(ui, |ui| {
            ui.style_mut().override_font_id = Some(MEDIUM);
            ui.label(rt_base(format!("{:^15}", "Name"))).highlight();
            ui.label(rt_base(format!("{:^8}", "PID"))).highlight();
            if ui.button(rt_base(format!("{:^10}", "VRAM"))).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::VRAM);
            }
            if ui.button(rt_base(format!("{:^5}", "GFX"))).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::GFX);
            }
            if ui.button(rt_base("Compute")).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::Compute);
            }
            if ui.button(rt_base(format!("{:^5}", "DMA"))).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::DMA);
            }
            if ui.button(rt_base("Decode")).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::Decode);
            }
            if ui.button(rt_base("Encode")).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::Encode);
            }
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
        ui.style_mut().override_font_id = Some(MEDIUM);
        let sensors = &self.buf_data.sensors;
        let y_fmt = |_y: f64, _range: &RangeInclusive<f64>| {
            String::new()
        };
        egui::Grid::new("Sensors").show(ui, |ui| {
            for (history, val, label, min, max, unit) in [
                (
                    &self.buf_data.sensors_history.sclk,
                    sensors.sclk,
                    "GFX_SCLK",
                    self.app_device_info.min_gpu_clk,
                    self.app_device_info.max_gpu_clk,
                    "MHz",
                ),
                (
                    &self.buf_data.sensors_history.mclk,
                    sensors.mclk,
                    "GFX_MCLK",
                    self.app_device_info.min_mem_clk,
                    self.app_device_info.max_mem_clk,
                    "MHz",
                ),
                (
                    &self.buf_data.sensors_history.vddgfx,
                    sensors.vddgfx,
                    "VDDGFX",
                    500, // "500 mV" is not an exact value
                    1500, // "1500 mV" is not an exact value
                    "mV",
                ),
                (
                    &self.buf_data.sensors_history.temp,
                    sensors.temp.map(|v| v.saturating_div(1000)),
                    "GFX Temp.",
                    0,
                    sensors.critical_temp.unwrap_or(105), // "105 C" is not an exact value
                    "C",
                ),
                (
                    &self.buf_data.sensors_history.power,
                    sensors.power,
                    "GFX Power",
                    0,
                    sensors.power_cap.unwrap_or(350), // "350 W" is not an exact value
                    "W",
                ),
                (
                    &self.buf_data.sensors_history.fan_rpm,
                    sensors.fan_rpm,
                    "Fan",
                    0,
                    sensors.fan_max_rpm.unwrap_or(6000), // "6000 RPM" is not an exact value
                    "RPM",
                ),
            ] {
                let Some(val) = val else { continue };

                ui.label(format!("{label}\n({val:4} {unit})"));

                if min == max {
                    ui.end_row();
                    continue;
                }

                let label_fmt = move |_name: &str, val: &PlotPoint| {
                    format!("{:.1}s\n{:.0} {unit}", val.x, val.y)
                };
                let points: PlotPoints = history.iter()
                    .map(|(i, val)| [i, val as f64]).collect();
                let line = Line::new(points).fill(1.0);
                Plot::new(label)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .show_axes([false, true])
                    .include_y(min)
                    .include_y(max)
                    .y_axis_formatter(y_fmt)
                    .label_formatter(label_fmt)
                    .auto_bounds_x()
                    .height(PLOT_HEIGHT * 1.5)
                    .width(PLOT_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
                ui.end_row();
            }
        });
        ui.label(format!(
            "PCI Link Speed => Gen{cur_gen}x{cur_width:<2} (Max. Gen{max_gen}x{max_width})",
            cur_gen = sensors.cur.gen,
            cur_width = sensors.cur.width,
            max_gen = sensors.max.gen,
            max_width = sensors.max.width,
        ));
    }

    fn egui_gpu_metrics_v1(&self, ui: &mut egui::Ui) {
        let gpu_metrics = &self.buf_data.gpu_metrics;

        if let Some(socket_power) = gpu_metrics.get_average_socket_power() {
            if socket_power != u16::MAX {
                ui.label(&format!("Socket Power => {socket_power:3} W"));
            }
        }

        ui.horizontal(|ui| {
            v1_helper(ui, "C", &[
                (gpu_metrics.get_temperature_edge(), "Edge"),
                (gpu_metrics.get_temperature_hotspot(), "Hotspot"),
                (gpu_metrics.get_temperature_mem(), "Memory"),
            ]);
        });

        ui.horizontal(|ui| {
            v1_helper(ui, "C", &[
                (gpu_metrics.get_temperature_vrgfx(), "VRGFX"),
                (gpu_metrics.get_temperature_vrsoc(), "VRSOC"),
                (gpu_metrics.get_temperature_vrmem(), "VRMEM"),
            ]);
        });

        ui.horizontal(|ui| {
            v1_helper(ui, "mV", &[
                (gpu_metrics.get_voltage_soc(), "SoC"),
                (gpu_metrics.get_voltage_gfx(), "GFX"),
                (gpu_metrics.get_voltage_mem(), "Mem"),
            ]);
        });

        for (avg, cur, name) in [
            (
                gpu_metrics.get_average_gfxclk_frequency(),
                gpu_metrics.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                gpu_metrics.get_average_socclk_frequency(),
                gpu_metrics.get_current_socclk(),
                "SOCCLK",
            ),
            (
                gpu_metrics.get_average_uclk_frequency(),
                gpu_metrics.get_current_uclk(),
                "UMCCLK",
            ),
            (
                gpu_metrics.get_average_vclk_frequency(),
                gpu_metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                gpu_metrics.get_average_dclk_frequency(),
                gpu_metrics.get_current_dclk(),
                "DCLK",
            ),
            (
                gpu_metrics.get_average_vclk1_frequency(),
                gpu_metrics.get_current_vclk1(),
                "VCLK1",
            ),
            (
                gpu_metrics.get_average_dclk1_frequency(),
                gpu_metrics.get_current_dclk1(),
                "DCLK1",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz"));
        }

        // Only Aldebaran (MI200) supports it.
        if let Some(hbm_temp) = gpu_metrics.get_temperature_hbm().and_then(|hbm_temp|
            (!hbm_temp.contains(&u16::MAX)).then_some(hbm_temp)
        ) {
            ui.horizontal(|ui| {
                ui.label("HBM Temp. (C) => [");
                for v in &hbm_temp {
                    let v = v.saturating_div(100);
                    ui.label(RichText::new(format!("{v:>5},")));
                }
                ui.label("]");
            });
        }
    }

    fn egui_gpu_metrics_v2(&self, ui: &mut egui::Ui) {
        const CORE_TEMP_LABEL: &str = "Core Temp (C)";
        const CORE_POWER_LABEL: &str = "Core Power (mW)";
        const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
        const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
        const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

        let gpu_metrics = &self.buf_data.gpu_metrics;

        ui.horizontal(|ui| {
            ui.label("GFX =>");
            let temp_gfx = gpu_metrics.get_temperature_gfx().map(|v| v.saturating_div(100));
            v2_helper(ui, &[
                (temp_gfx, "C"),
                (gpu_metrics.get_average_gfx_power(), "mW"),
                (gpu_metrics.get_current_gfxclk(), "MHz"),
            ]);
        });

        ui.horizontal(|ui| {
            ui.label("SoC =>");
            let temp_soc = gpu_metrics.get_temperature_soc().map(|v| v.saturating_div(100));
            v2_helper(ui, &[
                (temp_soc, "C"),
                (gpu_metrics.get_average_soc_power(), "mW"),
                (gpu_metrics.get_current_socclk(), "MHz"),
            ]);
        });

        if let Some(socket_power) = gpu_metrics.get_average_socket_power() {
            if socket_power != u16::MAX {
                ui.label(&format!("Socket Power => {socket_power:3} W"));
            }
        }

        for (avg, cur, name) in [
            (
                gpu_metrics.get_average_uclk_frequency(),
                gpu_metrics.get_current_uclk(),
                "UMCCLK",
            ),
            (
                gpu_metrics.get_average_fclk_frequency(),
                gpu_metrics.get_current_fclk(),
                "FCLK",
            ),
            (
                gpu_metrics.get_average_vclk_frequency(),
                gpu_metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                gpu_metrics.get_average_dclk_frequency(),
                gpu_metrics.get_current_dclk(),
                "DCLK",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz"));
        }

        let for_array = |ui: &mut egui::Ui, val: &[u16]| {
            for v in val {
                let v = if v == &u16::MAX { &0 } else { v };
                ui.label(RichText::new(format!("{v:>5},")));
            }
        };

        egui::Grid::new("GPU Metrics v2.x Core/L3").show(ui, |ui| {
            let temp_core = gpu_metrics.get_temperature_core()
                .map(|array| array.map(|v| v.saturating_div(100)));
            let temp_l3 = gpu_metrics.get_temperature_l3()
                .map(|array| array.map(|v| v.saturating_div(100)));

            for (val, label) in [
                (temp_core, CORE_TEMP_LABEL),
                (gpu_metrics.get_average_core_power(), CORE_POWER_LABEL),
                (gpu_metrics.get_current_coreclk(), CORE_CLOCK_LABEL),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label("=> [");
                for_array(ui, &val);
                ui.label("]");
                ui.end_row();
            }

            for (val, label) in [
                (temp_l3, L3_TEMP_LABEL),
                (gpu_metrics.get_current_l3clk(), L3_CLOCK_LABEL),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label("=> [");
                for_array(ui, &val);
                ui.label("]");
                ui.end_row();
            }
        });
    }
}

fn v1_helper(ui: &mut egui::Ui, unit: &str, v: &[(Option<u16>, &str)]) {
    for (val, name) in v {
        let v = check_metrics_val(*val);
        ui.label(format!("{name} => {v:>4} {unit}, "));
    }
}

fn v2_helper(ui: &mut egui::Ui, v: &[(Option<u16>, &str)]) {
    for (val, unit) in v {
        let v = check_metrics_val(*val);
        ui.label(format!("{v:>5} {unit}, "));
    }
}

fn label(text: &str, font: FontId) -> egui::Label {
    egui::Label::new(RichText::new(text).font(font)).sense(egui::Sense::click())
}

fn collapsing_plot(
    ui: &mut egui::Ui,
    text: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    use egui::{collapsing_header::CollapsingState, Id};

    let mut state = CollapsingState::load_with_default_open(ui.ctx(), Id::new(text), default_open);

    let _ = ui.horizontal(|ui| {
        let icon = {
            let text = if state.is_open() { "\u{25be}" } else { "\u{25b8}" };
            label(text, BASE)
        };
        let header = label(text, BASE);
        if ui.add(icon).clicked() || ui.add(header).clicked() {
            state.toggle(ui);
        }
    });

    state.show_body_unindented(ui, body);
}

fn collapsing(
    ui: &mut egui::Ui,
    text: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    use egui::{collapsing_header::CollapsingState, Id};

    let mut state = CollapsingState::load_with_default_open(ui.ctx(), Id::new(text), default_open);

    let header_res = ui.horizontal(|ui| {
        let icon = {
            let text = if state.is_open() { "\u{25be}" } else { "\u{25b8}" };
            label(text, HEADING)
        };
        let header = label(text, HEADING);
        if ui.add(icon).clicked() || ui.add(header).clicked() {
            state.toggle(ui);
        }
    });

    state.show_body_indented(&header_res.response, ui, body);
}

fn rt_base<T: Into<String>>(s: T) -> RichText {
    RichText::new(s.into()).font(BASE)
}
