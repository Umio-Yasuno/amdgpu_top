use crate::egui;
use crate::{collapsing, collapsing_plot, fl, label, MyApp, HEADING, BASE, SPACE, SPACE_3X};
use crate::gui_device_info::{GuiInfo, GuiConnectorInfo, GuiHwIpInfo, GuiIpDiscovery, GuiVbiosInfo, GuiVideoCapsInfo, GuiXdnaInfo};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MainTab {
    #[default]
    Info,
    GRBM,
    Activity,
    Sensors,
    GpuMetrics,
    Xdna,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InfoTab {
    #[default]
    DeviceInfo,
    IpDiscoveryTable,
    ConnectorInfo,
}

impl MyApp {
    pub(crate) fn egui_tab_gui(&mut self, ui: &mut egui::Ui) {
        let has_dec_enc_info = self.buf_data.device_info.decode.is_some()
            && self.buf_data.device_info.encode.is_some();
        let has_hw_ip_info = !self.buf_data.device_info.hw_ip_info_list.is_empty();

        {
            let vis = ui.visuals_mut();
            vis.striped = true;
        }

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.main_tab, MainTab::Info, fl!("info"));
            ui.separator();

            if !self.no_pc {
                ui.selectable_value(&mut self.main_tab, MainTab::GRBM, fl!("grbm"));
                ui.separator();
            }

            ui.selectable_value(&mut self.main_tab, MainTab::Activity, fl!("activity"));
            ui.separator();
            ui.selectable_value(&mut self.main_tab, MainTab::Sensors, fl!("sensor"));
            ui.separator();
            ui.selectable_value(&mut self.main_tab, MainTab::GpuMetrics, fl!("gpu_metrics"));
            ui.separator();

            if self.buf_data.xdna_device_path.is_some() {
                ui.selectable_value(&mut self.main_tab, MainTab::Xdna, "XDNA");
                ui.separator();
            }
        });

        ui.horizontal(|ui| {
            {
                let s = ui.style_mut();
                s.override_font_id = Some(BASE);
            }
            match self.main_tab {
                MainTab::Info => {
                    ui.separator();
                    ui.selectable_value(&mut self.info_tab, InfoTab::DeviceInfo, fl!("device_info"));

                    if !self.buf_data.device_info.ip_die_entries.is_empty() {
                        ui.separator();
                        ui.selectable_value(
                            &mut self.info_tab,
                            InfoTab::IpDiscoveryTable,
                            fl!("ip_discovery_table"),
                        );
                    }

                    if !self.buf_data.vec_connector_info.is_empty() {
                        ui.separator();
                        ui.selectable_value(
                            &mut self.info_tab,
                            InfoTab::ConnectorInfo,
                            fl!("connector_info"),
                        );
                    }

                    ui.separator();
                },
                _ => {},
            }
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            {
                let s = ui.spacing_mut();
                s.item_spacing = egui::vec2(12.0, 6.0);
            }
            match self.main_tab {
                MainTab::Info => {
                    self.sub_info_tab(ui, has_hw_ip_info, has_dec_enc_info);
                },
                MainTab::GRBM => {
                    ui.add(label(&fl!("grbm"), HEADING));
                    self.egui_perf_counter(
                        ui,
                        "GRBM",
                        &self.buf_data.stat.grbm,
                        &self.buf_data.history.grbm_history,
                    );
                    ui.add(label(&fl!("grbm2"), HEADING));
                    self.egui_perf_counter(
                        ui,
                        "GRBM2",
                        &self.buf_data.stat.grbm2,
                        &self.buf_data.history.grbm2_history,
                    );
                },
                MainTab::Activity => {
                    collapsing_plot(
                        ui,
                        &fl!("activity"),
                        true,
                        |ui| { self.egui_activity(ui) },
                    );

                    ui.add_space(SPACE);
                    ui.add(label(&fl!("vram"), HEADING));
                    self.egui_vram(ui);

                    ui.add_space(SPACE);
                    ui.add(label(&fl!("fdinfo"), HEADING));
                    self.egui_grid_fdinfo(ui);
                },
                MainTab::Sensors => if self.buf_data.stat.sensors.is_some() {
                    ui.add(label(&fl!("sensor"), HEADING));
                    self.egui_sensors(ui);
                },
                MainTab::GpuMetrics => {
                    self.egui_gpu_metrics(ui);
                    collapsing(
                        ui,
                        &fl!("throttling_log"),
                        false,
                        |ui| for (time, value) in self.buf_data.history.throttling_history.iter() {
                            ui.label(format!("{time:.1}s: {:?}", value.get_all_throttler()));
                        }
                    );
                },
                MainTab::Xdna => if self.buf_data.xdna_device_path.is_some() {
                    ui.add(label(&fl!("xdna_info"), HEADING));
                    self.buf_data.xdna_info(ui);

                    ui.add_space(SPACE_3X);
                    collapsing(
                        ui,
                        &fl!("xdna_fdinfo"),
                        true,
                        |ui| self.egui_grid_xdna_fdinfo(ui),
                    );
                },
            }
        });
    }

    fn sub_info_tab(&self, ui: &mut egui::Ui, has_hw_ip_info: bool, has_dec_enc_info: bool) {
        match self.info_tab {
            InfoTab::DeviceInfo => {
                ui.add(label(&fl!("device_info"), HEADING));
                self.buf_data.device_info.ui(
                    ui,
                    &self.wgpu_adapter_info,
                    &self.rocm_version,
                );

                if has_hw_ip_info {
                    ui.add_space(SPACE_3X);
                    ui.add(label(&fl!("hw_ip_info"), HEADING));
                    self.buf_data.device_info.hw_ip_info_list.ui(ui);
                }

                if has_dec_enc_info {
                    ui.add_space(SPACE_3X);
                    ui.add(label(&fl!("video_caps_info"), HEADING));
                    (
                        &self.buf_data.device_info.decode.unwrap(),
                        &self.buf_data.device_info.encode.unwrap(),
                    ).ui(ui);
                }

                if let Some(vbios) = &self.buf_data.device_info.vbios {
                    ui.add_space(SPACE_3X);
                    ui.add(label(&fl!("vbios_info"), HEADING));
                    vbios.ui(ui);
                }
            },
            InfoTab::IpDiscoveryTable => if !self.buf_data.device_info.ip_die_entries.is_empty() {
                ui.add_space(SPACE);
                ui.add(label(&fl!("ip_discovery_table"), HEADING));
                self.buf_data.device_info.ip_die_entries.ui(ui);
            },
            InfoTab::ConnectorInfo => if !self.buf_data.vec_connector_info.is_empty() {
                ui.add_space(SPACE);
                ui.add(label(&fl!("connector_info"), HEADING));
                for conn in &self.buf_data.vec_connector_info {
                    conn.ui(ui);
                }
            },
        }
    }
}
