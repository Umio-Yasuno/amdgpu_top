use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::ops::Range;
use std::path::PathBuf;
use eframe::egui;
use egui::{FontFamily, FontId, RichText, util::History};
use i18n_embed::DesktopLanguageRequester;

use libamdgpu_top::AMDGPU::{
    GpuMetrics,
    MetricsInfo,
};
use libamdgpu_top::{AppDeviceInfo, DevicePath, Sampling};
use libamdgpu_top::app::{AppAmdgpuTop, AppAmdgpuTopStat, AppOption};
use libamdgpu_top::stat::{self, FdInfoUsage, PerfCounter};

mod app;
use app::{GuiGpuMetrics, MyApp};

mod gui_device_info;
use gui_device_info::{GuiInfo, GuiIpDiscovery, GuiVbiosInfo, GuiVideoCapsInfo};

mod util;
use util::*;

mod localize;
pub use localize::LANGUAGE_LOADER;
use localize::localizer;

const SPACE: f32 = 8.0;
const BASE: FontId = FontId::new(14.0, FontFamily::Monospace);
const MEDIUM: FontId = FontId::new(15.0, FontFamily::Monospace);
const HEADING: FontId = FontId::new(16.0, FontFamily::Monospace);
const HISTORY_LENGTH: Range<usize> = 0..30; // seconds

#[derive(Clone)]
pub struct HistoryData {
    pub grbm_history: Vec<History<u8>>,
    pub grbm2_history: Vec<History<u8>>,
    pub fdinfo_history: History<FdInfoUsage>,
    pub sensors_history: SensorsHistory,
    pub pcie_bw_history: History<(u64, u64)>,
}

#[derive(Clone)]
pub struct GuiAppData {
    pub stat: AppAmdgpuTopStat,
    pub device_info: AppDeviceInfo,
    pub support_pcie_bw: bool,
    pub history: HistoryData,
}

impl GuiAppData {
    fn update_history(&mut self, secs: f64) {
        if let Some(arc_pcie_bw) = &self.stat.arc_pcie_bw {
            let lock = arc_pcie_bw.try_lock();
            if let Ok(pcie_bw) = lock {
                if let (Some(sent), Some(rec), Some(mps)) = (
                    pcie_bw.sent,
                    pcie_bw.received,
                    pcie_bw.max_payload_size,
                ) {
                    let sent = (sent * mps as u64) >> 20;
                    let rec = (rec * mps as u64) >> 20;
                    self.history.pcie_bw_history.add(secs, (sent, rec));
                }
            }
        }

        for (pc, history) in [
            (&self.stat.grbm, &mut self.history.grbm_history),
            (&self.stat.grbm2, &mut self.history.grbm2_history),
        ] {
            for ((_name, pos), h) in pc.index.iter().zip(history.iter_mut()) {
                h.add(secs, pc.bits.get(*pos));
            }
        }

        self.history.sensors_history.add(secs, &self.stat.sensors);
        self.history.fdinfo_history.add(secs, self.stat.fdinfo.fold_fdinfo_usage());
    }
}

pub fn run(
    app_name: &str,
    title_with_version: &str,
    device_path_list: &[DevicePath],
    selected_instance_number: u32,
    update_process_index_interval: u64,
) {
    let localizer = localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for library_fluent {error}");
    }

    let mut vec_app: Vec<_> = device_path_list.iter().filter_map(|device_path| {
        let amdgpu_dev = device_path.init().ok()?;

        let mut app = AppAmdgpuTop::new(
            amdgpu_dev,
            device_path.clone(),
            &AppOption { pcie_bw: true },
        )?;

        app.stat.grbm.get_i18n_index(&LANGUAGE_LOADER);
        app.stat.grbm2.get_i18n_index(&LANGUAGE_LOADER);

        Some(app)
    }).collect();

    {
        let t_index: Vec<(_, Arc<Mutex<Vec<_>>>)> = vec_app.iter().map(|app|
            (
                app.device_path.clone(),
                app.stat.arc_proc_index.clone(),
            )
        ).collect();

        stat::spawn_update_index_thread(t_index, update_process_index_interval);
    }

    let mut vec_data: Vec<_> = vec_app.iter().map(|app| {
        let fdinfo_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let sensors_history = SensorsHistory::default();
        let pcie_bw_history: History<(u64, u64)> = History::new(HISTORY_LENGTH, f32::INFINITY);
        let [grbm_history, grbm2_history] = [&app.stat.grbm, &app.stat.grbm2].map(|pc| {
            vec![History::<u8>::new(HISTORY_LENGTH, f32::INFINITY); pc.index.len()]
        });

        GuiAppData {
            stat: app.stat.clone(),
            device_info: app.device_info.clone(),
            support_pcie_bw: app.stat.arc_pcie_bw.is_some(),
            history: HistoryData {
                grbm_history,
                grbm2_history,
                fdinfo_history,
                sensors_history,
                pcie_bw_history,
            },
        }
    }).collect();

    let sample = Sampling::low();
    let device_list = device_path_list.iter().flat_map(DeviceListMenu::new).collect();
    let command_path = std::fs::read_link("/proc/self/exe").unwrap_or(PathBuf::from(app_name));

    let Some(data) = vec_data.iter().find(|&d| selected_instance_number == d.device_info.instance_number) else { return };

    let mut app = MyApp {
        device_list,
        command_path,
        fdinfo_sort: Default::default(),
        reverse_sort: false,
        buf_data: data.clone(),
        arc_data: Arc::new(Mutex::new(vec_data.clone())),
        show_sidepanel: true,
        gl_vendor_info: None,
        selected_instance_number,
    };

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1080.0, 840.0)),
        app_id: Some(app_name.to_string()),
        ..Default::default()
    };

    {
        let now = std::time::Instant::now();
        let share_data = app.arc_data.clone();

        std::thread::spawn(move || loop {
            for _ in 0..sample.count {
                for app in vec_app.iter_mut() {
                    app.update_pc();
                }

                std::thread::sleep(sample.delay);
            }

            for app in vec_app.iter_mut() {
                app.update(sample.to_duration());
            }

            for (app, data) in vec_app.iter_mut().zip(vec_data.iter_mut()) {
                data.stat = app.stat.clone();
                data.update_history(now.elapsed().as_secs_f64());
                app.clear_pc();
            }

            {
                let lock = share_data.lock();
                if let Ok(mut share_data) = lock {
                    *share_data = vec_data.clone();
                }
            }
        });
    }

    eframe::run_native(
        title_with_version,
        options,
        Box::new(|cc| {
            use eframe::glow::HasContext;
            use crate::egui::FontDefinitions;
            use crate::egui::FontData;

            if let Some(ctx) = &cc.gl {
                let ver = ctx.version().vendor_info.trim_start_matches("(Core Profile) ");
                app.gl_vendor_info = Some(ver.to_string());
            }

            let mut fonts = FontDefinitions::default();

            fonts.font_data.insert(
                "BIZUDGothic".to_string(),
                FontData::from_static(include_bytes!("../fonts/BIZUDGothic-Regular.ttf")),
            );

            fonts.families.get_mut(&FontFamily::Proportional).unwrap()
                .insert(3, "BIZUDGothic".to_owned());
            fonts.families.get_mut(&FontFamily::Monospace).unwrap()
                .insert(3, "BIZUDGothic".to_owned());

            cc.egui_ctx.set_fonts(fonts);

            Box::new(app)
        }),
    ).unwrap_or_else(|err| {
        eprintln!("{}", fl!("failed_to_set_up_gui"));
        eprintln!("{err}");
        panic!();
    });
}

impl MyApp {
    fn egui_device_list(&mut self, ui: &mut egui::Ui) {
        let Some(selected) = self.device_list
            .iter()
            .find(|&device| self.selected_instance_number == device.instance) else { return };

        egui::ComboBox::from_id_source("Device List Senpai")
            .selected_text(selected.to_string())
            .show_ui(ui, |ui| {
                for device in &self.device_list {
                    if selected.instance == device.instance { continue }

                    ui.selectable_value(
                        &mut self.selected_instance_number,
                        device.instance,
                        device.to_string(),
                    );
                }
            });
    }

    fn egui_side_panel(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(SPACE);
            collapsing(
                ui,
                &fl!("device_info"),
                true,
                |ui| self.buf_data.device_info.ui(ui, &self.gl_vendor_info),
            );

            if !self.buf_data.device_info.ip_die_entries.is_empty() {
                ui.add_space(SPACE);
                collapsing(
                    ui,
                    &fl!("ip_discovery_table"),
                    false,
                    |ui| self.buf_data.device_info.ip_die_entries.ui(ui),
                );
            }

            if let (Some(dec), Some(enc)) = (&self.buf_data.device_info.decode, &self.buf_data.device_info.encode) {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("video_caps_info"), false, |ui| (dec, enc).ui(ui));
            }

            if let Some(vbios) = &self.buf_data.device_info.vbios {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("vbios_info"), false, |ui| vbios.ui(ui));
            }

            ui.add_space(SPACE);
        });
    }

    fn egui_central_panel(&mut self, ui: &mut egui::Ui) {
        // ui.set_min_width(540.0);
        egui::ScrollArea::both().show(ui, |ui| {
            collapsing(ui, &fl!("grbm"), true, |ui| self.egui_perf_counter(
                ui,
                "GRBM",
                &self.buf_data.stat.grbm,
                &self.buf_data.history.grbm_history,
            ));
            ui.add_space(SPACE);
            collapsing(ui, &fl!("grbm2"), true, |ui| self.egui_perf_counter(
                ui,
                "GRBM2",
                &self.buf_data.stat.grbm2,
                &self.buf_data.history.grbm2_history,
            ));
            ui.add_space(SPACE);
            collapsing(ui, &fl!("vram"), true, |ui| self.egui_vram(ui));
            ui.add_space(SPACE);
            collapsing(ui, &fl!("fdinfo"), true, |ui| self.egui_grid_fdinfo(ui));
            ui.add_space(SPACE);
            collapsing(ui, &fl!("sensor"), true, |ui| self.egui_sensors(ui));

            if self.buf_data.support_pcie_bw {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("pcie_bw"), true, |ui| self.egui_pcie_bw(ui));
            }

            if let Some(metrics) = &self.buf_data.stat.metrics {
                let header = if let Some(h) = metrics.get_header() {
                    format!(
                        "{} v{}.{}",
                        fl!("gpu_metrics"),
                        h.format_revision,
                        h.content_revision
                    )
                } else {
                    String::new()
                };

                match metrics {
                    GpuMetrics::V1_0(_) |
                    GpuMetrics::V1_1(_) |
                    GpuMetrics::V1_2(_) |
                    GpuMetrics::V1_3(_) => {
                        ui.add_space(SPACE);
                        collapsing(ui, &header, true, |ui| metrics.v1_ui(ui));
                    },
                    GpuMetrics::V2_0(_) |
                    GpuMetrics::V2_1(_) |
                    GpuMetrics::V2_2(_) |
                    GpuMetrics::V2_3(_) |
                    GpuMetrics::V2_4(_) => {
                        ui.add_space(SPACE);
                        collapsing(ui, &header, true, |ui| metrics.v2_ui(ui));
                    },
                    _ => {},
                }
            }

            ui.add_space(SPACE);
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        {
            let lock = self.arc_data.try_lock();
            if let Ok(vec_data) = lock {
                let data = vec_data
                    .iter()
                    .find(|&d| self.selected_instance_number == d.device_info.instance_number)
                    .unwrap_or_else(|| {
                        eprintln!("invalid instance number: {}", self.selected_instance_number);
                        panic!();
                    });

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
                ui.toggle_value(&mut self.show_sidepanel, RichText::new(fl!("info"))
                    .font(BASE))
                    .on_hover_text(fl!("toggle_side_panel"));
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

use i18n_embed::fluent::FluentLanguageLoader;

trait I18nPerfCounter {
    fn get_i18n_index(&mut self, loader: &FluentLanguageLoader);
}

impl I18nPerfCounter for PerfCounter {
    fn get_i18n_index(&mut self, loader: &FluentLanguageLoader) {
        for (ref mut name, _) in self.index.iter_mut() {
            *name = loader.get(&name.replace(' ', "_").replace('/', ""));
        }
    }
}
