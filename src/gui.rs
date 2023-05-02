use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::PathBuf;
use eframe::egui;
use egui::{FontFamily, FontId, RichText, util::History};

use libdrm_amdgpu_sys::AMDGPU::{
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
use libdrm_amdgpu_sys::PCI;
use crate::{args::MainOpt, misc, stat, stat::FdInfoUsage, stat::Sensors, DevicePath, Sampling};
use stat::{FdInfoSortType, FdInfoView, PerfCounter, VramUsageView};

const SPACE: f32 = 8.0;
const BASE: FontId = FontId::new(14.0, FontFamily::Monospace);
const MEDIUM: FontId = FontId::new(15.0, FontFamily::Monospace);
const HEADING: FontId = FontId::new(16.0, FontFamily::Monospace);
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
    fn new(device_path: &DevicePath, pci: &PCI::BUS_INFO) -> Option<Self> {
        let instance = device_path.get_instance_number()?;

        let name = {
            let amdgpu_dev = device_path.init_with_option()?;
            amdgpu_dev.get_marketing_name().unwrap_or_default()
        };

        Some(Self {
            instance,
            pci: *pci,
            name,
        })
    }
}

pub fn egui_run(main_opt: MainOpt) {
    let self_pid = 0; // no filtering in GUI
    let device_path = DevicePath::from_main_opt(&main_opt);
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
    let mut grbm_history = vec![History::new(0..30, f32::INFINITY); grbm.index.len()];
    let mut grbm2_history = vec![History::new(0..30, f32::INFINITY); grbm2.index.len()];
    let mut fdinfo_history = History::new(0..30, f32::INFINITY);

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
    };

    let app_device_info = AppDeviceInfo::new(&amdgpu_dev, &ext_info, &memory_info, &pci_bus);
    let device_list = misc::get_device_path_list().iter().flat_map(|(device, pci)| {
        DeviceListMenu::new(device, pci)
    }).collect();
    let command_path = std::fs::read_link("/proc/self/exe")
        .unwrap_or(PathBuf::from(env!("CARGO_PKG_NAME")));

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
            std::thread::sleep(Duration::from_secs(main_opt.update_process_index));

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
                    fdinfo_history.add(
                        sec,
                        fdinfo.fold_fdinfo_usage(),
                    );
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

fn collapsing(
    ui: &mut egui::Ui,
    text: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new(text),
        default_open,
    );

    let label = |text: &str| -> egui::Label {
        egui::Label::new(RichText::new(text).font(HEADING)).sense(egui::Sense::click())
    };

    let header_res = ui.horizontal(|ui| {
        let icon = {
            let text = if state.is_open() { "\u{25be}" } else { "\u{25b8}" };
            label(text)
        };
        let header = label(text);
        if ui.add(icon).clicked() || ui.add(header).clicked() {
            state.toggle(ui);
        }
    });

    state.show_body_indented(&header_res.response, ui, |ui| body(ui));
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
                    "GRBM",
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
    vram_usage: VramUsageView,
    fdinfo: FdInfoView,
    fdinfo_history: History<FdInfoUsage>,
    gpu_metrics: GpuMetrics,
    sensors: Sensors,
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
}

impl AppDeviceInfo {
    fn new(
        amdgpu_dev: &DeviceHandle,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
        pci_bus: &PCI::BUS_INFO,
    ) -> Self {
        let (min_gpu_clk, max_gpu_clk) =
            amdgpu_dev.get_min_max_gpu_clock().unwrap_or((0, 0));
        let (min_mem_clk, max_mem_clk) =
            amdgpu_dev.get_min_max_memory_clock().unwrap_or((0, 0));
        let resizable_bar = memory_info.check_resizable_bar();
        let marketing_name = amdgpu_dev.get_marketing_name().unwrap_or_default();
        let hw_ip_info = HW_IP_LIST.iter().filter_map(|ip_type|
            amdgpu_dev.get_hw_ip_info(*ip_type).ok()
        ).filter(|hw_ip_info| hw_ip_info.count != 0).collect();

        Self {
            ext_info: ext_info.clone(),
            memory_info: memory_info.clone(),
            hw_ip_info,
            resizable_bar,
            min_gpu_clk,
            max_gpu_clk,
            min_mem_clk,
            max_mem_clk,
            marketing_name,
            pci_bus: *pci_bus,
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
        use egui::plot::{Line, Plot, PlotPoint, PlotPoints};
        use std::ops::RangeInclusive;

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
                (&self.buf_data.vram_usage.vram, "VRAM"),
                (&self.buf_data.vram_usage.gtt, "GTT"),
            ] {
                let progress = (v.usage >> 20) as f32 / (v.total >> 20) as f32;
                let text = format!("{:5} / {:5} MiB", v.usage >> 20, v.total >> 20);
                let bar = egui::ProgressBar::new(progress)
                    .text(RichText::new(&text).font(BASE));
                ui.label(RichText::new(name).font(MEDIUM));
                ui.add_sized([360.0, 16.0], bar);
                ui.end_row();
            }
        });
    }

    fn egui_grid_fdinfo(&mut self, ui: &mut egui::Ui) {
        use egui::plot::{Corner, Legend, Line, Plot, PlotPoint, PlotPoints};
        use std::ops::RangeInclusive;

        let y_fmt = |_y: f64, _range: &RangeInclusive<f64>| {
            String::new()
        };
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0}%", val.x, val.y)
        };

        let [mut gfx, mut compute, mut dma, mut dec, mut enc] = [
            Vec::<[f64; 2]>::with_capacity(30),
            Vec::<[f64; 2]>::with_capacity(30),
            Vec::<[f64; 2]>::with_capacity(30),
            Vec::<[f64; 2]>::with_capacity(30),
            Vec::<[f64; 2]>::with_capacity(30),
        ];

        for (i, usage) in self.buf_data.fdinfo_history.iter() {
            let usage_dec = usage.dec + usage.vcn_jpeg;
            let usage_enc = usage.enc + usage.uvd_enc;

            gfx.push([i, usage.gfx as f64]);
            compute.push([i, usage.compute as f64]);
            dma.push([i, usage.dma as f64]);
            dec.push([i, usage_dec as f64]);
            enc.push([i, usage_enc as f64]);
        }

        Plot::new("GFX Plot")
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
            }
            if ui.button(rt_base(format!("{:^5}", "GFX"))).clicked() {
                if let FdInfoSortType::GFX = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::GFX;
            }
            if ui.button(rt_base("Compute")).clicked() {
                if let FdInfoSortType::Compute = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::Compute;
            }
            if ui.button(rt_base(format!("{:^5}", "DMA"))).clicked() {
                if let FdInfoSortType::DMA = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::DMA;
            }
            if ui.button(rt_base("Decode")).clicked() {
                if let FdInfoSortType::Decode = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::Decode;
            }
            if ui.button(rt_base("Encode")).clicked() {
                if let FdInfoSortType::Encode = self.fdinfo_sort {
                    self.reverse_sort ^= true;
                } else {
                    self.reverse_sort = false;
                }
                self.fdinfo_sort = FdInfoSortType::Encode;
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
        let sensors = &self.buf_data.sensors;
        {
            ui.style_mut().override_font_id = Some(MEDIUM);
        }
        egui::Grid::new("Sensors").show(ui, |ui| {
            for (name, val, unit) in [
                ("GFX_SCLK", sensors.sclk, "MHz"),
                ("GFX_MCLK", sensors.mclk, "MHz"),
                ("VDDNB", sensors.vddnb, "mV"),
                ("VDDGFX", sensors.vddgfx, "mV"),
            ] {
                let Some(val) = val else { continue };
                ui.label(name);
                ui.label("=>");
                ui.label(format!("{val:5} {unit}"));
                ui.end_row();
            }
        });
        if let Some(temp) = sensors.temp {
            let temp = temp.saturating_div(1_000);
            if let Some(crit) = sensors.critical_temp {
                ui.label(format!("GPU Temp. => {temp:3} C (Crit. {crit} C)"));
            } else {
                ui.label(format!("GPU Temp. => {temp:3} C"));
            }
        }
        if let Some(power) = sensors.power {
            if let Some(cap) = sensors.power_cap {
                ui.label(format!("GPU Power => {power:3} W (Cap. {cap} W)"));
            } else {
                ui.label(format!("GPU Power => {power:3} W"));
            }
        }
        if let Some(fan_rpm) = sensors.fan_rpm {
            if let Some(max_rpm) = sensors.fan_max_rpm {
                ui.label(format!("Fan => {fan_rpm:4} RPM (Max. {max_rpm} RPM)"));
            } else {
                ui.label(format!("Fan => {fan_rpm:4} RPM"));
            }
        }
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

    fn egui_gpu_metrics_v2(&self, ui: &mut egui::Ui) {
        const CORE_TEMP_LABEL: &str = "Core Temp (C)";
        const CORE_POWER_LABEL: &str = "Core Power (mW)";
        const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
        const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
        const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

        let gpu_metrics = &self.buf_data.gpu_metrics;

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
