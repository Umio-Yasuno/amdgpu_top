use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::ops::Range;
use eframe::{egui, Theme};
use egui::{FontFamily, FontId, RichText, ViewportBuilder};
use i18n_embed::DesktopLanguageRequester;

use libamdgpu_top::{
    AMDGPU::{
        GpuMetrics,
        MetricsInfo,
    },
    app::{
        AppAmdgpuTop,
        AppOption,
    },
    stat::{
        self,
        PerfCounter,
    },
    AppDeviceInfo,
    DevicePath,
    Sampling,
    PCI,
};

mod app;
use app::{GuiAppData, GuiMemoryErrorCount, GuiGpuMetrics, MyApp};

mod gui_device_info;
use gui_device_info::{GuiInfo, GuiConnectorInfo, GuiHwIpInfo, GuiIpDiscovery, GuiVbiosInfo, GuiVideoCapsInfo};

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

pub fn run(
    app_name: &str,
    title_with_version: &str,
    device_path_list: &[DevicePath],
    selected_pci_bus: PCI::BUS_INFO,
    update_process_index_interval: u64,
    no_pc: bool,
    is_dark_mode: bool,
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
        let device_paths: Vec<DevicePath> = device_path_list.to_vec();
        stat::spawn_update_index_thread(device_paths, update_process_index_interval);
    }

    let (mut vec_data, vec_device_info): (Vec<_>, Vec<_>) = vec_app.iter().map(|app| (GuiAppData::new(app), app.device_info.clone())).unzip();

    let sample = Sampling::low();

    let data = vec_data
        .iter()
        .find(|&d| selected_pci_bus == d.pci_bus)
        .unwrap_or_else(|| {
            eprintln!("invalid PCI bus: {selected_pci_bus}");
            panic!();
        });
    let device_info = vec_device_info
        .iter()
        .find(|&d| selected_pci_bus == d.pci_bus)
        .unwrap_or_else(|| {
            eprintln!("invalid PCI bus: {selected_pci_bus}");
            panic!();
        })
        .clone();

    let mut app = MyApp {
        fdinfo_sort: Default::default(),
        reverse_sort: false,
        device_info,
        vec_device_info,
        buf_data: data.clone(),
        arc_data: Arc::new(Mutex::new(vec_data.clone())),
        show_sidepanel: true,
        gl_vendor_info: None,
        rocm_version: libamdgpu_top::get_rocm_version(),
        selected_pci_bus,
        no_pc,
        pause: false,
        full_fdinfo_list: false,
    };

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(egui::vec2(1080.0, 840.0))
            .with_app_id(app_name),
        default_theme: if is_dark_mode { Theme::Dark } else { Theme::Light },
        ..Default::default()
    };

    {
        let now = std::time::Instant::now();
        let share_data = app.arc_data.clone();

        std::thread::spawn(move || loop {
            if !no_pc {
                for _ in 0..sample.count {
                    for app in vec_app.iter_mut() {
                        app.update_pc();
                    }

                    std::thread::sleep(sample.delay);
                }
            } else {
                std::thread::sleep(sample.to_duration());
            }

            for app in vec_app.iter_mut() {
                app.update(sample.to_duration());
            }

            for (app, data) in vec_app.iter_mut().zip(vec_data.iter_mut()) {
                data.stat = app.stat.clone();
                data.update_history(now.elapsed().as_secs_f64(), no_pc);
                if !no_pc { app.clear_pc(); }
            }

            {
                let lock = share_data.lock();
                if let Ok(mut share_data) = lock {
                    share_data.clone_from(&vec_data);
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

            Ok(Box::new(app))
        }),
    ).unwrap_or_else(|err| {
        eprintln!("{}", fl!("failed_to_set_up_gui"));
        eprintln!("{err}");
        panic!();
    });
}

impl MyApp {
    fn egui_device_list(&mut self, ui: &mut egui::Ui) {
        let selected_text = self.device_info.menu_entry();

        egui::ComboBox::from_id_source("Device List")
            .selected_text(&selected_text)
            .show_ui(ui, |ui| for device in &self.vec_device_info {
                if self.device_info.pci_bus == device.pci_bus {
                    let _ = ui.add_enabled(
                        false,
                        egui::SelectableLabel::new(true, &selected_text),
                    );
                } else {
                    ui.selectable_value(
                        &mut self.selected_pci_bus,
                        device.pci_bus,
                        device.menu_entry(),
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
                |ui| self.device_info.ui(ui, &self.gl_vendor_info, &self.rocm_version),
            );

            if !self.device_info.hw_ip_info_list.is_empty() {
                ui.add_space(SPACE);
                collapsing(
                    ui,
                    &fl!("hw_ip_info"),
                    false,
                    |ui| self.device_info.hw_ip_info_list.ui(ui),
                );
            }

            if !self.device_info.ip_die_entries.is_empty() {
                ui.add_space(SPACE);
                collapsing(
                    ui,
                    &fl!("ip_discovery_table"),
                    false,
                    |ui| self.device_info.ip_die_entries.ui(ui),
                );
            }

            if let (Some(dec), Some(enc)) = (&self.device_info.decode, &self.device_info.encode) {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("video_caps_info"), false, |ui| (dec, enc).ui(ui));
            }

            if let Some(vbios) = &self.device_info.vbios {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("vbios_info"), false, |ui| vbios.ui(ui));
            }

            if !self.buf_data.vec_connector_info.is_empty() {
                ui.add_space(SPACE);

                collapsing(ui, &fl!("connector_info"), false, |ui| {
                    for conn in &self.buf_data.vec_connector_info {
                        conn.ui(ui);
                    }
                });
            }

            ui.add_space(SPACE);
        });
    }

    fn egui_central_panel(&mut self, ui: &mut egui::Ui) {
        // ui.set_min_width(540.0);
        egui::ScrollArea::both().show(ui, |ui| {
            if !self.no_pc {
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
            }

            collapsing(ui, &fl!("vram"), true, |ui| self.egui_vram(ui));
            ui.add_space(SPACE);
            collapsing(ui, &fl!("fdinfo"), true, |ui| self.egui_grid_fdinfo(ui));

            if self.buf_data.stat.sensors.is_some() {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("sensor"), true, |ui| self.egui_sensors(ui));
            }

            if self.buf_data.support_pcie_bw {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("pcie_bw"), true, |ui| self.egui_pcie_bw(ui));
            }

            if let Some(ecc) = &self.buf_data.stat.memory_error_count {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("ecc_memory_error_count"), true, |ui| ecc.ui(ui));
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
                    GpuMetrics::V1_3(_) |
                    GpuMetrics::V1_4(_) |
                    GpuMetrics::V1_5(_) => {
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
                    GpuMetrics::V3_0(_) => {
                        ui.add_space(SPACE);
                        collapsing(ui, &header, true, |ui| metrics.v3_ui(ui));
                    },
                    _ => {},
                }
            }

            collapsing(ui, &fl!("throttling_log"), false, |ui| {
                for (time, value) in self.buf_data.history.throttling_history.iter() {
                    ui.label(format!("{time:.1}s: {:?}", value.get_all_throttler()));
                }
            });

            ui.add_space(SPACE);
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.pause {
            let lock = self.arc_data.try_lock();
            if let Ok(vec_data) = lock {
                self.buf_data = vec_data
                    .iter()
                    .find(|&d| self.selected_pci_bus == d.pci_bus)
                    .unwrap_or_else(|| {
                        eprintln!("invalid PCI bus: {}", self.selected_pci_bus);
                        panic!();
                    })
                    .clone();
            }

            if self.selected_pci_bus != self.device_info.pci_bus {
                self.device_info = self.vec_device_info
                    .iter()
                    .find(|&d| self.selected_pci_bus == d.pci_bus)
                    .unwrap()
                    .clone();
            }
        }

        let visuals;
        {
            let mut style = (*ctx.style()).clone();
            style.override_font_id = Some(BASE);
            visuals = style.visuals.clone();
            ctx.set_style(style);
        }

        ctx.clear_animations();

        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(theme) = visuals.light_dark_small_toggle_button(ui) {
                    ctx.set_visuals(theme);
                };
                ui.toggle_value(
                    &mut self.show_sidepanel,
                    RichText::new(fl!("info")).font(BASE)).on_hover_text(fl!("toggle_side_panel"),
                );
                self.egui_device_list(ui);

                {
                    ui.separator();
                    egui::gui_zoom::zoom_menu_buttons(ui);
                    ui.label(format!("{:>3.0}%", ui.ctx().zoom_factor() * 100.0));
                }

                {
                    ui.separator();
                    ui.toggle_value(
                        &mut self.pause,
                        RichText::new(fl!("pause")).font(BASE),
                    );
                }
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
