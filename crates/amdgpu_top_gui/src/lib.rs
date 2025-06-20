use std::sync::{Arc, Mutex, LazyLock};
use std::time::Duration;
use std::ops::Range;
use eframe::egui;
use egui::{FontFamily, FontId, Theme, RichText, ViewportBuilder};
use egui::viewport::ViewportCommand;
use i18n_embed::DesktopLanguageRequester;

use libamdgpu_top::{
    AMDGPU::{
        GpuMetrics,
        MetricsInfo,
    },
    app::AppAmdgpuTop,
    stat::{
        self,
        FdInfoSortType,
        PerfCounter,
    },
    AppDeviceInfo,
    DevicePath,
    GuiWgpuBackend,
    Sampling,
    UiArgs,
    PCI,
};

mod gui_app_data;
use gui_app_data::GuiAppData;

mod app;
use app::{GuiMemoryErrorCount, MyApp};

mod gui_gpu_metrics;
use gui_gpu_metrics::GuiGpuMetrics;

mod gui_device_info;
use gui_device_info::{GuiInfo, GuiConnectorInfo, GuiHwIpInfo, GuiIpDiscovery, GuiVbiosInfo, GuiVideoCapsInfo, GuiXdnaInfo};

mod tab_gui;

mod util;
use util::*;

mod localize;
pub use localize::LANGUAGE_LOADER;
use localize::localizer;

const SPACE: f32 = 8.0;
const SPACE_3X: f32 = SPACE * 3.0;
// const SMALL: FontId = FontId::new(12.0, FontFamily::Monospace);
const BASE: FontId = FontId::new(14.0, FontFamily::Monospace);
const MEDIUM: FontId = FontId::new(15.0, FontFamily::Monospace);
// const LARGE: FontId = FontId::new(16.0, FontFamily::Monospace);
const HEADING: FontId = FontId::new(16.0, FontFamily::Monospace);
const HISTORY_LENGTH: Range<usize> = 0..30; // seconds
static SIDE_PANEL_STATE_ID: LazyLock<egui::Id> = LazyLock::new(|| {
    egui::Id::new("side_panel_state")
});
static THEME_ID: LazyLock<egui::Id> = LazyLock::new(|| {
    // Light, Dark, System
    egui::Id::new("theme_v2")
});
static SIDE_PANEL_ID: LazyLock<egui::Id> = LazyLock::new(|| {
    egui::Id::new("side_panel")
});
static PCI_BUS_ID: LazyLock<egui::Id> = LazyLock::new(|| {
    egui::Id::new("pci_bus")
});

pub fn run(
    app_name: &str,
    title_with_version: &str,
    UiArgs {
        selected_device_path,
        device_path_list,
        update_process_index,
        no_pc,
        is_dark_mode,
        gui_wgpu_backend,
        tab_gui,
        ..
    }: UiArgs,
) {
    let selected_pci_bus = selected_device_path.pci;
    let localizer = localizer();
    let requested_languages = DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!("Error while loading languages for library_fluent {error}");
    }

    let (mut vec_app, mut suspended_devices) = AppAmdgpuTop::create_app_and_suspended_list(
        &device_path_list,
        &Default::default(),
    );

    for app in vec_app.iter_mut() {
        app.stat.grbm.get_i18n_index(&LANGUAGE_LOADER);
        app.stat.grbm2.get_i18n_index(&LANGUAGE_LOADER);
    }

    {
        let mut device_paths: Vec<DevicePath> = device_path_list.clone();

        if let Some(xdna_device_path) = vec_app
            .iter()
            .find_map(|app| app.xdna_device_path.as_ref())
        {
            device_paths.push(xdna_device_path.clone());
        }

        stat::spawn_update_index_thread(device_paths, update_process_index);
    }

    let mut vec_data: Vec<_> = vec_app.iter().map(GuiAppData::new).collect();

    let sample = Sampling::low();

    let selected_pci_bus = if !vec_data.iter().any(|d| selected_pci_bus == d.pci_bus) {
        vec_data.first().unwrap().pci_bus
    } else {
        selected_pci_bus
    };

    let data = vec_data
        .iter()
        .find(|&d| selected_pci_bus == d.pci_bus)
        .unwrap_or_else(|| {
            eprintln!("invalid PCI bus: {selected_pci_bus}");
            panic!();
        })
        .clone();

    let mut gui_app = MyApp {
        fdinfo_sort: if data.device_info.is_apu {
            FdInfoSortType::GTT
        } else {
            Default::default()
        },
        reverse_sort: false,
        buf_data: data,
        buf_vec_data: vec_data.clone(),
        arc_data: Arc::new(Mutex::new(vec_data.clone())),
        device_path_list,
        show_sidepanel: true,
        wgpu_adapter_info: None,
        rocm_version: libamdgpu_top::get_rocm_version(),
        selected_pci_bus,
        no_pc,
        pause: false,
        full_fdinfo_list: false,
        tab_gui,
        main_tab: Default::default(),
        info_tab: Default::default(),
    };

    unsafe {
        // In the case of the Vulkan backend, this app may wake up the suspended devices.
        let backend = match gui_wgpu_backend {
            GuiWgpuBackend::Gl => "opengl",
            GuiWgpuBackend::Vulkan => "vulkan",
        };

        std::env::set_var("WGPU_BACKEND", backend);
        // use APU if it is available
        std::env::set_var("WGPU_POWER_PREF", "low");
    }

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(egui::vec2(1080.0, 840.0))
            .with_app_id(app_name),
        ..Default::default()
    };

    {
        let now = std::time::Instant::now();
        let share_data = gui_app.arc_data.clone();

        std::thread::spawn(move || loop {
            if !no_pc {
                for _ in 0..sample.count {
                    for app in vec_app.iter_mut() {
                        app.update_pc();
                    }

                    std::thread::sleep(sample.delay);
                }

                for app in vec_app.iter_mut() {
                    app.update_pc_usage();
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

            suspended_devices.retain(|dev| {
                let is_active = dev.check_if_device_is_active();

                if is_active {
                    let Ok(amdgpu_dev) = dev.init() else { return true };
                    let Some(app) = AppAmdgpuTop::new(
                        amdgpu_dev,
                        dev.clone(),
                        &Default::default(),
                    ) else { return true };
                    vec_data.push(GuiAppData::new(&app));
                    vec_app.push(app);
                }

                !is_active
            });
        });
    }

    eframe::run_native(
        title_with_version,
        options,
        Box::new(move |cc| {
            use egui::FontDefinitions;
            use egui::FontData;

            if let Some(render_state) = &cc.wgpu_render_state {
                gui_app.wgpu_adapter_info = render_state.available_adapters
                    .iter()
                    .find_map(|adapter| {
                        let mut info = adapter.get_info();
                        if let Some((_, driver_ver)) = info
                            .driver_info
                            .rfind("Mesa")
                            .and_then(|idx| info.driver_info.split_at_checked(idx))
                        {
                            info.driver_info = driver_ver.to_string();
                        }

                        if info.vendor == 0x1002 { Some(info) } else { None }
                    })
                    .clone();
            }

            {
                let mut fonts = FontDefinitions::default();

                fonts.font_data.insert(
                    "BIZUDGothic".to_string(),
                    Arc::new(FontData::from_static(include_bytes!("../fonts/BIZUDGothic-Regular.ttf"))),
                );

                fonts.families.get_mut(&FontFamily::Proportional).unwrap()
                    .insert(3, "BIZUDGothic".to_owned());
                fonts.families.get_mut(&FontFamily::Monospace).unwrap()
                    .insert(3, "BIZUDGothic".to_owned());

                cc.egui_ctx.set_fonts(fonts);
            }

            {
                let id = *SIDE_PANEL_STATE_ID;
                let s: Option<bool> = cc.egui_ctx.data_mut(|id_map| id_map.get_persisted(id));

                if let Some(s) = s {
                    gui_app.show_sidepanel = s;
                }
            }

            {
                let id = *PCI_BUS_ID;
                let s: Option<String> = cc.egui_ctx.data_mut(|id_map| id_map.get_persisted(id));

                if let Some(pci_bus) = s.and_then(|s| s.parse::<PCI::BUS_INFO>().ok()) {
                    if gui_app.buf_vec_data.iter().any(|d| pci_bus == d.pci_bus) {
                        gui_app.selected_pci_bus = pci_bus;
                    }
                }
            }

            if let Some(is_dark_mode) = is_dark_mode {
                let theme = if is_dark_mode { Theme::Dark } else { Theme::Light };

                cc.egui_ctx.data_mut(|id_map| {
                    let v = id_map.get_persisted_mut_or_insert_with(
                        *THEME_ID,
                        || { theme },
                    );
                    *v = theme;
                });

                cc.egui_ctx.set_theme(theme);
            } else {
                let id = *THEME_ID;
                let theme: Option<Theme> = cc.egui_ctx.data_mut(|id_map| id_map.get_persisted(id));

                if let Some(theme) = theme {
                    cc.egui_ctx.set_theme(theme);
                }
            }

            Ok(Box::new(gui_app))
        }),
    ).unwrap_or_else(|err| {
        eprintln!("{}", fl!("failed_to_set_up_gui"));
        eprintln!("{err}");
        panic!();
    });
}

impl MyApp {
    fn egui_device_list(&mut self, ui: &mut egui::Ui) {
        let selected_text = self.buf_data.device_info.menu_entry();

        egui::ComboBox::from_id_salt("Device List")
            .selected_text(&selected_text)
            .show_ui(ui, |ui| for device in &self.device_path_list {
                if self.buf_data.device_info.pci_bus == device.pci {
                    let _ = ui.add_enabled(
                        false,
                        egui::SelectableLabel::new(true, &selected_text),
                    );
                } else if self.buf_vec_data.iter().any(|data| data.pci_bus == device.pci) {
                    ui.selectable_value(
                        &mut self.selected_pci_bus,
                        device.pci,
                        device.menu_entry(),
                    );
                } else {
                    let label = format!("{} ({})", device.menu_entry(), fl!("suspended"));
                    let _ = ui.add_enabled(
                        false,
                        egui::SelectableLabel::new(false, label),
                    );
                }
            });
    }

    fn egui_side_panel(&self, ui: &mut egui::Ui) {
        let scroll_area = if self.tab_gui {
            egui::ScrollArea::new([false, false])
        } else {
            egui::ScrollArea::vertical()
        };

        {
            let vis = ui.visuals_mut();
            vis.striped = true;
        }

        scroll_area.show(ui, |ui| {
            ui.add_space(SPACE);
            collapsing(
                ui,
                &fl!("device_info"),
                true,
                |ui| self.buf_data.device_info.ui(ui, &self.wgpu_adapter_info, &self.rocm_version),
            );

            if self.buf_data.xdna_device_path.is_some() {
                ui.add_space(SPACE);
                collapsing(
                    ui,
                    &fl!("xdna_info"),
                    true,
                    |ui| self.buf_data.xdna_info(ui),
                );
            }

            if !self.buf_data.device_info.hw_ip_info_list.is_empty() {
                ui.add_space(SPACE);
                collapsing(
                    ui,
                    &fl!("hw_ip_info"),
                    false,
                    |ui| self.buf_data.device_info.hw_ip_info_list.ui(ui),
                );
            }

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
            collapsing(ui, &fl!("activity"), true, |ui| self.egui_activity(ui));
            ui.add_space(SPACE);
            collapsing(ui, &fl!("fdinfo"), true, |ui| self.egui_grid_fdinfo(ui));

            if self.buf_data.xdna_device_path.is_some() {
                ui.add_space(SPACE);
                collapsing(ui, &fl!("xdna_fdinfo"), true, |ui| self.egui_grid_xdna_fdinfo(ui));
            }

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

            if self.buf_data.stat.metrics.is_some() {
                ui.add_space(SPACE);
                self.egui_gpu_metrics(ui);
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
            {
                let lock = self.arc_data.try_lock();
                if let Ok(vec_data) = lock {
                    self.buf_vec_data.clone_from(&vec_data);
                }
            }

            self.buf_data = self.buf_vec_data
                .iter()
                .find(|&d| self.selected_pci_bus == d.pci_bus)
                .unwrap_or_else(|| {
                    eprintln!("invalid PCI bus: {}", self.selected_pci_bus);
                    panic!();
                })
                .clone();
        }

        {
            use egui::{Key, KeyboardShortcut, Modifiers};
            pub const CLOSE_KEY: KeyboardShortcut =
                KeyboardShortcut::new(Modifiers::CTRL, Key::Q);

            if ctx.input_mut(|i| i.consume_shortcut(&CLOSE_KEY)) {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }

        {
            let mut style = (*ctx.style()).clone();
            if self.tab_gui {
                style.override_font_id = Some(MEDIUM);
            } else {
                style.override_font_id = Some(BASE);
            }
            ctx.set_style(style);
        }

        ctx.clear_animations();

        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                {
                    let pre_pci_bus = self.selected_pci_bus;

                    self.egui_device_list(ui);

                    if pre_pci_bus != self.selected_pci_bus {
                        let cur_pci_bus = self.selected_pci_bus.to_string();

                        ctx.data_mut(|id_map| {
                            let v = id_map.get_persisted_mut_or_insert_with(
                                *PCI_BUS_ID,
                                || { cur_pci_bus.clone() },
                            );
                            *v = cur_pci_bus;
                        });
                    }
                    ui.separator();
                }

                if !self.tab_gui {
                    let res =
                        ui.toggle_value(
                            &mut self.show_sidepanel,
                            RichText::new(fl!("info"))
                        ).on_hover_text(fl!("toggle_side_panel"),
                    );

                    if res.changed() {
                        ctx.data_mut(|id_map| {
                            let v = id_map.get_persisted_mut_or_insert_with(
                                *SIDE_PANEL_STATE_ID,
                                || { self.show_sidepanel },
                            );
                            *v = self.show_sidepanel;
                        });
                    }
                    ui.separator();
                }

                {
                    let pre_theme = ctx.theme();

                    if self.tab_gui {
                        egui::widgets::global_theme_preference_switch(ui);
                    } else {
                        egui::widgets::global_theme_preference_buttons(ui);
                    }

                    let cur_theme = ctx.theme();

                    if pre_theme != cur_theme {
                        ctx.data_mut(|id_map| {
                            let v = id_map.get_persisted_mut_or_insert_with(
                                *THEME_ID,
                                || { cur_theme },
                            );
                            *v = cur_theme;
                        });
                    }
                    ui.separator();
                }

                if self.tab_gui {
                    if ui.button("-").clicked() {
                        egui::gui_zoom::zoom_out(ctx);
                    }
                    if ui.button("+").clicked() {
                        egui::gui_zoom::zoom_in(ctx);
                    }
                    if ui.button("↻").clicked() {
                        ctx.set_zoom_factor(1.0);
                    }
                    ui.label(format!("🔍 {:>3.0}%", ctx.zoom_factor() * 100.0));
                    ui.separator();
                }

                ui.toggle_value(
                    &mut self.pause,
                    RichText::new(fl!("pause")),
                );

                ui.separator();
                if !self.tab_gui {
                    ui.toggle_value(&mut self.tab_gui, "Tab Mode");
                } else {
                    ui.toggle_value(&mut self.tab_gui, "Single");
                }
                ui.separator();
            });

            if !self.tab_gui { ui.horizontal(|ui| {
                egui::gui_zoom::zoom_menu_buttons(ui);
                ui.label(format!("{:>3.0}%", ctx.zoom_factor() * 100.0));
                ui.separator();

                quit_button(ctx, ui);
            }); }
        });

        if !self.tab_gui && self.show_sidepanel {
            egui::SidePanel::left(*SIDE_PANEL_ID).show(ctx, |ui| self.egui_side_panel(ui));
        }

        egui::CentralPanel::default().show(
            ctx,
            |ui| if self.tab_gui {
                self.egui_tab_gui(ui)
            } else {
                self.egui_central_panel(ui)
            }
        );

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

fn quit_button(ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui.button(RichText::new(fl!("quit") + " (Ctrl+Q)")).clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    };
}

use i18n_embed::fluent::FluentLanguageLoader;

trait I18nPerfCounter {
    fn get_i18n_index(&mut self, loader: &FluentLanguageLoader);
}

impl I18nPerfCounter for PerfCounter {
    fn get_i18n_index(&mut self, loader: &FluentLanguageLoader) {
        for pc_index in self.pc_index.iter_mut() {
            pc_index.name = loader.get(&pc_index.name.replace(' ', "_").replace('/', ""));
        }
    }
}
