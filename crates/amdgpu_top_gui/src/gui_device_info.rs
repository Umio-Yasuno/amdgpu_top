use std::fmt::Write;
use eframe::egui;
use crate::{
    app::grid,
    AppDeviceInfo,
    util::*,
    fl,
};

use libamdgpu_top::{ConnectorInfo, ModeProp, drmModeModeInfo, drmModePropType};
use libamdgpu_top::AMDGPU::{
    GPU_INFO,
    HW_IP::HwIpInfo,
    IpDieEntry,
    VBIOS::VbiosInfo,
    VIDEO_CAPS::VideoCapsInfo,
};

pub trait GuiVideoCapsInfo {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiVideoCapsInfo for (&VideoCapsInfo, &VideoCapsInfo) {
    fn ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("codec_info").show(ui, |ui| {
            ui.label(fl!("codec")).highlight();
            ui.label(fl!("decode")).highlight();
            ui.label(fl!("encode")).highlight();
            ui.end_row();

            let n_a = fl!("n_a");

            for (name, decode, encode) in [
                ("MPEG2", self.0.mpeg2, self.1.mpeg2),
                ("MPEG4", self.0.mpeg4, self.1.mpeg4),
                ("VC1", self.0.vc1, self.1.vc1),
                ("MPEG4_AVC", self.0.mpeg4_avc, self.1.mpeg4_avc),
                ("HEVC", self.0.hevc, self.1.hevc),
                ("JPEG", self.0.jpeg, self.1.jpeg),
                ("VP9", self.0.vp9, self.1.vp9),
                ("AV1", self.0.av1, self.1.av1),
            ] {
                ui.label(name);
                if let Some(dec) = decode {
                    ui.label(format!("{}x{}", dec.max_width, dec.max_height));
                } else {
                    ui.label(&n_a);
                }
                if let Some(enc) = encode {
                    ui.label(format!("{}x{}", enc.max_width, enc.max_height));
                } else {
                    ui.label(&n_a);
                }
                ui.end_row();
            }
        });

    }
}

pub trait GuiHwIpInfo {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiHwIpInfo for Vec<HwIpInfo> {
    fn ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("hw_ip_info").show(ui, |ui| {
            ui.label(fl!("ip_type")).highlight();
            ui.label(fl!("count")).highlight();
            ui.label(fl!("version")).highlight();
            ui.label(fl!("queues")).highlight();
            ui.end_row();

            for hw_ip_info in self {
                ui.label(hw_ip_info.ip_type.to_string());
                ui.label(hw_ip_info.count.to_string());
                ui.label(
                    format!("{:2}.{}",
                    hw_ip_info.info.hw_ip_version_major,
                    hw_ip_info.info.hw_ip_version_minor,
                ));
                ui.label(hw_ip_info.info.num_queues().to_string());
                ui.end_row();
            }
        });
    }
}

pub trait GuiIpDiscovery {
    fn ui(&self, ui: &mut egui::Ui);

    fn per_die(ip_die_entry: &IpDieEntry, ui: &mut egui::Ui) {
        egui::Grid::new(format!("ip_discovery_table die{}", ip_die_entry.die_id)).show(ui, |ui| {
            ui.label(fl!("ip_hw")).highlight();
            ui.label(fl!("version")).highlight();
            ui.label(fl!("num")).highlight();
            ui.end_row();

            for ip_hw in &ip_die_entry.ip_hw_ids {
                let hw_id = ip_hw.hw_id.to_string();
                let Some(inst) = ip_hw.instances.first() else { continue };
                ui.label(hw_id);
                ui.label(format!("{}.{}.{}", inst.major, inst.minor, inst.revision));
                ui.label(ip_hw.instances.len().to_string());
                ui.end_row();
            }
        });
    }
}

impl GuiIpDiscovery for Vec<IpDieEntry> {
    fn ui(&self, ui: &mut egui::Ui) {
        let gpu_die = fl!("gpu_die");
        for die in self.iter() {
            let label = format!("{gpu_die}: {}", die.die_id);
            collapsing(ui, &label, false, |ui| Self::per_die(die, ui));
        }
    }
}

pub trait GuiVbiosInfo {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiVbiosInfo for VbiosInfo {
    fn ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("vbios_info").show(ui, |ui| {
            for (name, val) in [
                (fl!("vbios_name"), &self.name),
                (fl!("vbios_pn"), &self.pn),
                (fl!("vbios_version"), &self.ver),
                (fl!("vbios_date"), &self.date),
                (fl!("vbios_size"), &self.size.to_string()),
            ] {
                ui.label(name).highlight();
                ui.label(val);
                ui.end_row();
            }
        });
    }
}

pub trait GuiInfo {
    fn ui(&self, ui: &mut egui::Ui, gl_vendor_info: &Option<String>, rocm_version: &Option<String>);
    fn device_info(&self, ui: &mut egui::Ui, gl_vendor_info: &Option<String>, rocm_version: &Option<String>);
    fn gfx_info(&self, ui: &mut egui::Ui);
    fn memory_info(&self, ui: &mut egui::Ui);
    fn cache_info(&self, ui: &mut egui::Ui);
    fn power_cap_info(&self, ui: &mut egui::Ui);
    fn temp_info(&self, ui: &mut egui::Ui);
    fn fan_info(&self, ui: &mut egui::Ui);
    fn link_info(&self, ui: &mut egui::Ui);
}

impl GuiInfo for AppDeviceInfo {
    fn ui(
        &self,
        ui: &mut egui::Ui,
        gl_vendor_info: &Option<String>,
        rocm_version: &Option<String>,
    ) {
        egui::Grid::new("app_device_info").show(ui, |ui| {
            self.device_info(ui, gl_vendor_info, rocm_version);
            self.gfx_info(ui);
            self.memory_info(ui);
            self.cache_info(ui);
            self.power_cap_info(ui);
            self.temp_info(ui);
            self.fan_info(ui);
            self.link_info(ui);

            let profiles: Vec<String> = self.power_profiles.iter().map(|p| p.to_string()).collect();

            ui.label(fl!("supported_power_profiles").to_string());
            ui.label(format!("{profiles:#?}"));
            ui.end_row();
        });
    }

    fn device_info(
        &self,
        ui: &mut egui::Ui,
        gl_vendor_info: &Option<String>,
        rocm_version: &Option<String>,
    ) {
        let dev_id = format!("{:#0X}.{:#0X}", self.ext_info.device_id(), self.ext_info.pci_rev_id());

        grid(ui, &[
            (&fl!("device_name"), &self.marketing_name),
            (&fl!("pci_bus"), &self.pci_bus.to_string()),
            (&fl!("did_rid"), &dev_id),
        ]);

        if let Some(gl) = gl_vendor_info {
            ui.label(&fl!("opengl_driver_ver"));
            ui.label(gl);
            ui.end_row();
        }

        if let Some(rocm) = rocm_version {
            ui.label(&fl!("rocm_ver"));
            ui.label(rocm);
            ui.end_row();
        }

        if let Some(ver) = &self.gfx_target_version {
            ui.label(&fl!("gfx_target_version"));
            ui.label(ver);
            ui.end_row();
        }

        ui.end_row();
    }

    fn gfx_info(&self, ui: &mut egui::Ui) {
        let gpu_type = if self.ext_info.is_apu() { fl!("apu") } else { fl!("dgpu") };
        let family = self.ext_info.get_family_name();
        let asic = self.ext_info.get_asic_name();
        let chip_class = self.ext_info.get_chip_class();
        let max_good_cu_per_sa = self.ext_info.get_max_good_cu_per_sa();
        let min_good_cu_per_sa = self.ext_info.get_min_good_cu_per_sa();
        let cu_per_sa = if max_good_cu_per_sa != min_good_cu_per_sa {
            format!("[{min_good_cu_per_sa}, {max_good_cu_per_sa}]")
        } else {
            max_good_cu_per_sa.to_string()
        };
        let rb_pipes = self.ext_info.rb_pipes();
        let rop_count = self.ext_info.calc_rop_count();
        let rb_type = if asic.rbplus_allowed() {
            fl!("rb_plus")
        } else {
            fl!("rb")
        };
        let peak_gp = format!("{} {}", rop_count * self.max_gpu_clk / 1000, fl!("gp_s"));
        let peak_fp32 = format!("{} {}", self.ext_info.peak_gflops(), fl!("gflops"));

        grid(ui, &[
            (&fl!("gpu_type"), &gpu_type),
            (&fl!("family"), &family.to_string()),
            (&fl!("asic_name"), &asic.to_string()),
            (&fl!("chip_class"), &chip_class.to_string()),
            (&fl!("shader_engine"), &self.ext_info.max_se().to_string()),
            (&fl!("shader_array_per_se"), &self.ext_info.max_sa_per_se().to_string()),
            (&fl!("cu_per_sa"), &cu_per_sa),
            (&fl!("total_cu"), &self.ext_info.cu_active_number().to_string()),
            (&rb_type, &format!("{rb_pipes} ({rop_count} ROPs)")),
            (&fl!("peak_gp"), &peak_gp),
            (&fl!("gpu_clock"), &format!("{}-{} MHz", self.min_gpu_clk, self.max_gpu_clk)),
            (&fl!("peak_fp32"), &peak_fp32),
        ]);
        ui.end_row();
    }

    fn memory_info(&self, ui: &mut egui::Ui) {
        let re_bar = if self.resizable_bar {
            fl!("enabled")
        } else {
            fl!("disabled")
        };
        let ecc = if self.ecc_memory {
            fl!("supported")
        } else {
            fl!("not_supported")
        };

        grid(ui, &[
            (&fl!("vram_type"), &self.ext_info.get_vram_type().to_string()),
            (&fl!("vram_bit_width"), &format!("{}-{}", self.ext_info.vram_bit_width, fl!("bit"))),
            (&fl!("vram_size"), &format!("{} {}", self.memory_info.vram.total_heap_size >> 20, fl!("mib"))),
            (&fl!("memory_clock"), &format!("{}-{} {}", self.min_mem_clk, self.max_mem_clk, fl!("mhz"))),
            (&fl!("resizable_bar"), &re_bar),
            (&fl!("ecc_memory"), &ecc),
        ]);
        ui.end_row();
    }

    fn cache_info(&self, ui: &mut egui::Ui) {
        let kib = fl!("kib");
        let mib = fl!("mib");
        let banks = fl!("banks");

        ui.label(fl!("l1_cache_per_cu"));
        ui.label(format!("{:4} {kib}", self.l1_cache_size_kib_per_cu));
        ui.end_row();
        if 0 < self.gl1_cache_size_kib_per_sa {
            ui.label(fl!("gl1_cache_per_sa"));
            ui.label(format!("{:4} {kib}", self.gl1_cache_size_kib_per_sa));
            ui.end_row();
        }
        ui.label(fl!("l2_cache"));
        ui.label(format!(
            "{:4} {kib} ({} {banks})",
            self.total_l2_cache_size_kib,
            self.actual_num_tcc_blocks,
        ));
        ui.end_row();
        if 0 < self.total_l3_cache_size_mib {
            ui.label(fl!("l3_cache"));
            ui.label(format!(
                "{:4} {mib} ({} {banks})",
                self.total_l3_cache_size_mib,
                self.actual_num_tcc_blocks,
            ));
            ui.end_row();
        }
        ui.end_row();
    }

    fn power_cap_info(&self, ui: &mut egui::Ui) {
        let Some(cap) = &self.power_cap else { return };

        ui.label(fl!("power_cap"));
        ui.label(format!("{:4} W ({}-{} W)", cap.current, cap.min, cap.max));
        ui.end_row();
        ui.label(fl!("power_cap_default"));
        ui.label(format!("{:4} W", cap.default));
        ui.end_row();
    }

    fn temp_info(&self, ui: &mut egui::Ui) {
        for temp in [
            &self.edge_temp,
            &self.junction_temp,
            &self.memory_temp,
        ] {
            let Some(temp) = temp else { continue };
            let name = temp.type_.to_string();
            if let Some(crit) = temp.critical {
                ui.label(format!("{name} Temp. (Critical)"));
                ui.label(format!("{crit:4} C"));
                ui.end_row();
            }
            if let Some(e) = temp.emergency {
                ui.label(format!("{name} Temp. (Emergency)"));
                ui.label(format!("{e:4} C"));
                ui.end_row();
            }
        }
    }

    fn fan_info(&self, ui: &mut egui::Ui) {
        let Some(fan_rpm) = &self.fan_max_rpm else { return };

        ui.label("Fan RPM (Max)");
        ui.label(format!("{fan_rpm:4} RPM"));
        ui.end_row();
    }

    fn link_info(&self, ui: &mut egui::Ui) {
        let pcie_link_speed = fl!("pcie_link_speed");
        let fl_max = fl!("max");
        let dpm = fl!("dpm");
        if let [Some(min), Some(max)] = [&self.min_dpm_link, &self.max_dpm_link] {
            ui.label(format!("{pcie_link_speed} ({dpm})"));
            ui.label(format!("Gen{}x{} - Gen{}x{}", min.gen, min.width, max.gen, max.width));
            ui.end_row();
        } else if let Some(max) = &self.max_dpm_link {
            ui.label(format!("{pcie_link_speed} ({dpm}, {fl_max})"));
            ui.label(format!("Gen{}x{}", max.gen, max.width));
            ui.end_row();
        }

        if let Some(gpu) = &self.max_gpu_link {
            ui.label(format!("{pcie_link_speed} ({}, {fl_max})", fl!("gpu")));
            ui.label(format!("Gen{}x{}", gpu.gen, gpu.width));
            ui.end_row();
        }

        if let Some(system) = &self.max_system_link {
            ui.label(format!("{pcie_link_speed} ({}, {fl_max})", fl!("system")));
            ui.label(format!("Gen{}x{}", system.gen, system.width));
            ui.end_row();
        }
    }
}

pub trait GuiConnectorInfo {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiConnectorInfo for ConnectorInfo {
    fn ui(&self, ui: &mut egui::Ui) {
        let title = self.name();
        collapsing(ui, &title, false, |ui| {
            if let Some(crtc) = self.crtc {
                collapsing_with_id(ui, "CRTC", &format!("CRTC {title}"), true, |ui| {
                    ui.label(format!(
                        "{}x{}@{}",
                        crtc.mode.hdisplay,
                        crtc.mode.vdisplay,
                        crtc.mode.refresh_rate(),
                    ));
                });
            }

            if !self.mode_info.is_empty() {
                collapsing_with_id(ui, "Modes", &format!("Modes {title}"), false, |ui| {
                    for mode in &self.mode_info {
                        mode.ui(ui);
                    }
                });
            }

            for mode_prop in &self.mode_props {
                mode_prop.ui(&title, ui);
            }
        });
    }
}

pub trait GuiModeInfo {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiModeInfo for drmModeModeInfo {
    fn ui(&self, ui: &mut egui::Ui) {
        let txt = format!(
            "{}x{}@{:.2}{}{}",
            self.hdisplay,
            self.vdisplay,
            self.refresh_rate(),
            if self.type_is_preferred() { " preferred" } else { "" },
            if self.type_is_driver() { " driver" } else { "" },
        );
        ui.label(txt);
        ui.end_row();
    }
}

pub trait GuiModeProp {
    fn ui(&self, conn_name: &str, ui: &mut egui::Ui);
}

impl GuiModeProp for &(ModeProp, u64) {
    fn ui(&self, conn_name: &str, ui: &mut egui::Ui) {
        collapsing_with_id(ui, &self.0.name, &format!("{} {conn_name}", &self.0.name), false, |ui| {
            egui::Grid::new(&self.0.name).show(ui, |ui| {
                ui.label("type");
                ui.label(self.0.prop_type.to_string());
                ui.end_row();

                ui.label("id");
                ui.label(self.0.prop_id.to_string());
                ui.end_row();

                ui.label("value");
                ui.label(self.1.to_string());
                ui.end_row();

                match self.0.prop_type {
                    drmModePropType::RANGE => {
                        ui.label("values");
                        ui.label(format!("{:?}", self.0.values));
                        ui.end_row();
                    },
                    drmModePropType::ENUM => {
                        let enums = self.0.enums.iter().fold(String::new(), |mut s, enum_| {
                            let _ = write!(s, "{:?}={}, ", enum_.name(), enum_.value);
                            s
                        });

                        ui.label("enums");
                        ui.label(format!("[{}]", enums.trim_end_matches(", ")));
                        ui.end_row();
                    },
                    _ => {},
                }
            });
        });
    }
}
