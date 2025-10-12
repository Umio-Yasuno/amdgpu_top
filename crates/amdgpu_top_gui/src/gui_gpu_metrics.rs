use std::fmt::Write;
use crate::{egui, GpuMetrics, MetricsInfo, fl};
use libamdgpu_top::stat::gpu_metrics_util::*;

pub trait GuiGpuMetrics: MetricsInfo {
    fn v1_ui(&self, ui: &mut egui::Ui);
    fn v2_ui(&self, ui: &mut egui::Ui);
    fn v3_ui(&self, ui: &mut egui::Ui);

    fn v1_4_v1_5_ui(&self, ui: &mut egui::Ui);

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

    fn socket_power(&self, ui: &mut egui::Ui);

    fn throttle_status(&self, ui: &mut egui::Ui) {
        if let Some(thr) = self.get_throttle_status_info() {
            ui.label(
                format!(
                    "{}: {:?}",
                    fl!("throttle_status"),
                    thr.get_all_throttler(),
                )
            );
        }
    }
}

impl GuiGpuMetrics for GpuMetrics {
    fn v1_ui(&self, ui: &mut egui::Ui) {
        self.socket_power(ui);

        ui.horizontal(|ui| {
            Self::v1_helper(ui, &fl!("c"), &[
                (self.get_temperature_vrgfx(), "VRGFX"),
                (self.get_temperature_vrsoc(), "VRSOC"),
                (self.get_temperature_vrmem(), "VRMEM"),
            ]);
        });

        ui.horizontal(|ui| {
            Self::v1_helper(ui, &fl!("mv"), &[
                (self.get_voltage_soc(), "SoC"),
                (self.get_voltage_gfx(), "GFX"),
                (self.get_voltage_mem(), "Mem"),
            ]);
        });

        let fl_avg = fl!("avg");
        let fl_cur = fl!("cur");
        let mhz = fl!("mhz");

        for (avg, cur, name) in [
            (
                self.get_average_gfxclk_frequency(),
                self.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                self.get_average_socclk_frequency(),
                self.get_current_socclk(),
                "SOCCLK",
            ),
            (
                self.get_average_uclk_frequency(),
                self.get_current_uclk(),
                "UMCCLK",
            ),
            (
                self.get_average_vclk_frequency(),
                self.get_current_vclk(),
                "VCLK",
            ),
            (
                self.get_average_dclk_frequency(),
                self.get_current_dclk(),
                "DCLK",
            ),
            (
                self.get_average_vclk1_frequency(),
                self.get_current_vclk1(),
                "VCLK1",
            ),
            (
                self.get_average_dclk1_frequency(),
                self.get_current_dclk1(),
                "DCLK1",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => {fl_avg} {avg:>4} {mhz}, {fl_cur} {cur:>4} {mhz}"));
        }

        // Only Aldebaran (MI200) supports it.
        if let Some(hbm_temp) = check_hbm_temp(self.get_temperature_hbm()) {
            ui.horizontal(|ui| {
                let s = hbm_temp.iter().fold(String::new(), |mut s, v| {
                    let _ = write!(s, "{v:>5},");
                    s
                });

                ui.label(format!("{} =>", fl!("hbm_temp")));
                ui.label(format!("[{s}]"));
            });
        }

        match self {
            GpuMetrics::V1_4(_) |
            GpuMetrics::V1_5(_) => self.v1_4_v1_5_ui(ui),
            _ => {},
        }

        self.throttle_status(ui);
    }

    fn v2_ui(&self, ui: &mut egui::Ui) {
        let mhz = fl!("mhz");
        let mw = fl!("mw");

        ui.horizontal(|ui| {
            ui.label(format!("{} => {pad:9}", fl!("cpu"), pad = ""));
            Self::v2_helper(ui, &[
                (self.get_average_cpu_power(), &mw),
            ]);
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} =>", fl!("gfx")));
            let temp_gfx = self.get_temperature_gfx().map(|v| v.saturating_div(100));
            Self::v2_helper(ui, &[
                (temp_gfx, "C"),
                (self.get_average_gfx_power(), &mw),
                (self.get_current_gfxclk(), &mhz),
            ]);
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} =>", fl!("soc")));
            let temp_soc = self.get_temperature_soc().map(|v| v.saturating_div(100));
            Self::v2_helper(ui, &[
                (temp_soc, "C"),
                (self.get_average_soc_power(), &mw),
                (self.get_current_socclk(), &mhz),
            ]);
        });

        /*
            Most APUs return `average_socket_power` in mW,
            but Renoir APU (Renoir, Lucienne, Cezanne, Barcelo) return in W
            depending on the power management firmware version.  

            ref: drivers/gpu/drm/amd/pm/swsmu/smu12/renoir_ppt.c
            ref: https://gitlab.freedesktop.org/drm/amd/-/issues/2321
        */
        // socket_power(ui, self);

        let fl_avg = fl!("avg");
        let fl_cur = fl!("cur");

        for (avg, cur, name) in [
            (
                self.get_average_gfxclk_frequency(),
                self.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                self.get_average_socclk_frequency(),
                self.get_current_socclk(),
                "SOCCLK",
            ),
            (
                self.get_average_uclk_frequency(),
                self.get_current_uclk(),
                "UMCCLK",
            ),
            (
                self.get_average_fclk_frequency(),
                self.get_current_fclk(),
                "FCLK",
            ),
            (
                self.get_average_vclk_frequency(),
                self.get_current_vclk(),
                "VCLK",
            ),
            (
                self.get_average_dclk_frequency(),
                self.get_current_dclk(),
                "DCLK",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => {fl_avg} {avg:>4} {mhz}, {fl_cur} {cur:>4} {mhz}"));
        }

        egui::Grid::new("GPU Metrics v2.x Core/L3").show(ui, |ui| {
            let core_temp = check_temp_array(self.get_temperature_core());
            let l3_temp = check_temp_array(self.get_temperature_l3());
            let [core_power, core_clk] = [
                self.get_average_core_power(),
                self.get_current_coreclk(),
            ].map(check_power_clock_array);
            let l3_clk = check_power_clock_array(self.get_current_l3clk());

            for (val, label) in [
                (core_temp, fl!("core_temp")),
                (core_power, fl!("core_power")),
                (core_clk, fl!("core_clock")),
            ] {
                let Some(val) = val else { continue };
                let s = val.iter().fold(String::new(), |mut s, v| {
                    let _ = write!(s, "{v:>5},");
                    s
                });
                ui.label(format!("{label} =>"));
                ui.label(format!("[{s}]"));
                ui.end_row();
            }

            for (val, label) in [
                (l3_temp, fl!("l3_temp")),
                (l3_clk, fl!("l3_clock")),
            ] {
                let Some(val) = val else { continue };
                let s = val.iter().fold(String::new(), |mut s, v| {
                    let _ = write!(s, "{v:>5},");
                    s
                });

                ui.label(format!("{label} =>"));
                ui.label(format!("[{s}]"));
                ui.end_row();
            }

            for (label, voltage, current) in [
                (
                    fl!("cpu"),
                    self.get_average_cpu_voltage(),
                    self.get_average_cpu_current(),
                ),
                (
                    fl!("soc"),
                    self.get_average_soc_voltage(),
                    self.get_average_soc_current(),
                ),
                (
                    fl!("gfx"),
                    self.get_average_gfx_voltage(),
                    self.get_average_gfx_current(),
                ),
            ] {
                let Some(voltage) = voltage else { continue };
                let Some(current) = current else { continue };

                ui.label(format!(
                    "{label} => {voltage:>5} {mv}, {current:>5} {ma}",
                    mv = fl!("mv"),
                    ma = fl!("ma"),
                ));
            }
        });

        self.throttle_status(ui);
    }

    fn v1_4_v1_5_ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("GPU Metrics v1.4/v1.5, clock, activity").show(ui, |ui| {
            if let Some(all_gfxclk) = self.get_all_instances_current_gfxclk() {
                ui.label("GFXCLK (Current) =>");
                ui.label(format!("[{}]", all_clk_helper(&all_gfxclk)));
                ui.end_row();
            }

            for (label, all_clk) in [
                ("SOCCLK (Current) =>", self.get_all_instances_current_socclk()),
                ("VCLK0 (Current)  =>", self.get_all_instances_current_vclk0()),
                ("DCLK0 (Current)  =>", self.get_all_instances_current_dclk0()),
            ] {
                let Some(all_clk) = all_clk else { continue };
                ui.label(label);
                ui.label(format!("[{}]", all_clk_helper(&all_clk)));
                ui.end_row();
            }

            if let Some(all_vcn) = self.get_all_vcn_activity() {
                ui.label("VCN Activity =>");
                ui.label(format!("[{}]", all_activity_helper(&all_vcn)));
                ui.end_row();
            }

            if let Some(all_jpeg) = self.get_all_jpeg_activity() {
                ui.label("JPEG Activity =>");
                ui.label(format!("[{}]", all_activity_helper(&all_jpeg)));
                ui.end_row();
            }
        });

        if let [Some(xgmi_width), Some(xgmi_speed)] = [
            self.get_xgmi_link_width(),
            self.get_xgmi_link_speed(),
        ] {
            ui.label(format!("XGMI => x{xgmi_width} {xgmi_speed}Gbps"));
        }
    }

    fn v3_ui(&self, ui: &mut egui::Ui) {
        let mhz = fl!("mhz");
        let mw = fl!("mw");

        ui.horizontal(|ui| {
            ui.label(format!("{} => {pad:9}", fl!("cpu"), pad = ""));
            Self::v2_helper(ui, &[
                (self.get_average_cpu_power(), &mw),
            ]);
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} =>", fl!("gfx")));
            let temp_gfx = self.get_temperature_gfx().map(|v| v.saturating_div(100));
            Self::v2_helper(ui, &[
                (temp_gfx, "C"),
                (self.get_average_gfx_power(), &mw),
                (self.get_current_gfxclk(), &mhz),
            ]);
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} =>", fl!("soc")));
            let temp_soc = self.get_temperature_soc().map(|v| v.saturating_div(100));
            Self::v2_helper(ui, &[
                (temp_soc, "C"),
                (self.get_average_soc_power(), &mw),
                (self.get_current_socclk(), &mhz),
            ]);
        });

        // self.socket_power(ui);

        if let [Some(dram_reads), Some(dram_writes)] = [
            self.get_average_dram_reads(),
            self.get_average_dram_writes(),
        ] {
            ui.label(format!(
                " DRAM => Reads: {dram_reads:>4} MB/s, Writes: {dram_writes:>} MB/s",
            ));
        }

        let fl_avg = fl!("avg");
        let fl_cur = fl!("cur");

        for (avg, cur, name) in [
            (
                self.get_average_gfxclk_frequency(),
                self.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                self.get_average_socclk_frequency(),
                self.get_current_socclk(),
                "SOCCLK",
            ),
            (
                self.get_average_uclk_frequency(),
                self.get_current_uclk(),
                "UMCCLK",
            ),
            (
                self.get_average_fclk_frequency(),
                self.get_current_fclk(),
                "FCLK",
            ),
            (
                self.get_average_vclk_frequency(),
                self.get_current_vclk(),
                "VCLK",
            ),
            (
                self.get_average_dclk_frequency(),
                self.get_current_dclk(),
                "DCLK",
            ),
            (
                self.get_average_vpeclk_frequency(),
                None,
                "VPECLK",
            ),
            (
                self.get_average_ipuclk_frequency(),
                None,
                "IPUCLK",
            ),
            (
                self.get_average_mpipu_frequency(),
                None,
                "MPIPUCLK",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => {fl_avg} {avg:>4} {mhz}, {fl_cur} {cur:>4} {mhz}"));
        }

        if let Some(ipu) = self.get_average_ipu_activity() {
            egui::Grid::new("GPU Metrics v3.x IPU").show(ui, |ui| {
                ui.label("IPU =>");
                ui.label(format!(" IPU => {ipu:?}%"));

                if let Some(ipu_power) = self.get_average_ipu_power() {
                    ui.label(format!(", {ipu_power:>5} {mw}"));
                }
                ui.end_row();

                ui.label("");


                if let [Some(ipu_reads), Some(ipu_writes)] = [
                    self.get_average_ipu_reads(),
                    self.get_average_ipu_writes(),
                ] {
                    ui.label(format!(
                        "        Reads: {ipu_reads:>4} MB/s, Writes: {ipu_writes:>} MB/s",
                    ));
                }
            });
        }

        egui::Grid::new("GPU Metrics v3.x Core").show(ui, |ui| {
            let core_temp = check_temp_array(self.get_temperature_core());
            let [core_power, core_clk] = [
                self.get_average_core_power(),
                self.get_current_coreclk(),
            ].map(check_power_clock_array);

            for (val, label) in [
                (core_temp, fl!("core_temp")),
                (core_power, fl!("core_power")),
                (core_clk, fl!("core_clock")),
            ] {
                let Some(val) = val else { continue };
                let s = val.iter().fold(String::new(), |mut s, v| {
                    let _ = write!(s, "{v:>5},");
                    s
                });
                ui.label(format!("{label} =>"));
                ui.label(format!("[{s}]"));
                ui.end_row();
            }
        });

        egui::Grid::new("GPU Metrics v3.x Throttling").show(ui, |ui| {
            // The use of `throttle_residency_*` is not documented,
            // and I don't have the actual gpu_metrics_v3_0 data.
            for (label, thr) in [
                ("PROCHOT", self.get_throttle_residency_prochot()),
                ("SPL", self.get_throttle_residency_spl()),
                ("FPPT", self.get_throttle_residency_fppt()),
                ("SPPT", self.get_throttle_residency_sppt()),
                ("THM_CORE", self.get_throttle_residency_thm_core()),
                ("THM_GFX", self.get_throttle_residency_thm_gfx()),
                ("THM_SOC", self.get_throttle_residency_thm_soc()),
            ] {
                let Some(thr) = thr else { continue };
                ui.label(format!("{label:<8}:"));
                ui.label(format!("{thr}"));
                ui.end_row();
            }
        });

        if let Some(stapm_limit) = self.get_stapm_power_limit()
            && stapm_limit != u16::MAX
            && let Some(current_stapm_limit) = self.get_current_stapm_power_limit()
            && current_stapm_limit != u16::MAX
        {
            ui.label(format!(
                " STAPM Limit: {stapm_limit:>5} mW, {current_stapm_limit:>5} mW (Current)"
            ));
        }
    }

    fn socket_power(&self, ui: &mut egui::Ui) {
        let avg = check_metrics_val(self.get_average_socket_power());
        ui.label(format!("{} => {avg:>3} W", fl!("socket_power")));

        match self {
            GpuMetrics::V1_4(_) |
            GpuMetrics::V1_5(_) => {
                let cur = check_metrics_val(self.get_current_socket_power());
                ui.label(format!("{} => {cur:>3} W", fl!("current_socket_power")));
            },
            _ => {},
        }
    }
}
