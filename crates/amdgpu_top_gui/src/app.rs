use std::fmt::Write;
use std::sync::{Arc, Mutex};
use eframe::egui;
use egui::{RichText, util::History};
use egui_plot::{Corner, Legend, Line, Plot, PlotPoint, PlotPoints};
use crate::{BASE, MEDIUM, HISTORY_LENGTH};

use libamdgpu_top::{AppDeviceInfo, ConnectorInfo, PCI};
use libamdgpu_top::app::{
    AppAmdgpuTop,
    AppAmdgpuTopStat,
};
use libamdgpu_top::AMDGPU::{RasErrorCount, MetricsInfo};
use libamdgpu_top::stat::{
    self,
    gpu_metrics_util::*,
    FdInfoUsage,
    FdInfoSortType,
    PerfCounter,
};

use crate::{GpuMetrics, util::*, fl};

const SPACING: [f32; 2] = [16.0; 2];

const SENSORS_HEIGHT: f32 = 96.0;
const SENSORS_WIDTH: f32 = SENSORS_HEIGHT * 4.0;

#[derive(Clone)]
pub struct HistoryData {
    pub grbm_history: Vec<History<u8>>,
    pub grbm2_history: Vec<History<u8>>,
    pub vram_history: History<u64>,
    pub gtt_history: History<u64>,
    pub fdinfo_history: History<FdInfoUsage>,
    pub sensors_history: SensorsHistory,
    pub pcie_bw_history: History<(u64, u64)>,
}

#[derive(Clone)]
pub struct GuiAppData {
    pub stat: AppAmdgpuTopStat,
    pub pci_bus: PCI::BUS_INFO,
    pub support_pcie_bw: bool,
    pub history: HistoryData,
    pub vec_connector_info: Vec<ConnectorInfo>,
}

impl GuiAppData {
    pub fn new(app: &AppAmdgpuTop) -> Self {
        let vram_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let gtt_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let fdinfo_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let sensors_history = SensorsHistory::default();
        let pcie_bw_history: History<(u64, u64)> = History::new(HISTORY_LENGTH, f32::INFINITY);
        let [grbm_history, grbm2_history] = [&app.stat.grbm, &app.stat.grbm2].map(|pc| {
            vec![History::<u8>::new(HISTORY_LENGTH, f32::INFINITY); pc.index.len()]
        });

        Self {
            stat: app.stat.clone(),
            pci_bus: app.device_info.pci_bus,
            support_pcie_bw: app.stat.arc_pcie_bw.is_some(),
            history: HistoryData {
                grbm_history,
                grbm2_history,
                vram_history,
                gtt_history,
                fdinfo_history,
                sensors_history,
                pcie_bw_history,
            },
            vec_connector_info: libamdgpu_top::connector_info(&app.device_path),
        }
    }

    pub fn update_history(&mut self, secs: f64, no_pc: bool) {
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

        if !no_pc {
            for (pc, history) in [
                (&self.stat.grbm, &mut self.history.grbm_history),
                (&self.stat.grbm2, &mut self.history.grbm2_history),
            ] {
                for ((_name, pos), h) in pc.index.iter().zip(history.iter_mut()) {
                    h.add(secs, pc.bits.get(*pos));
                }
            }
        }

        self.history.vram_history.add(secs, self.stat.vram_usage.0.vram.heap_usage);
        self.history.gtt_history.add(secs, self.stat.vram_usage.0.gtt.heap_usage);
        self.history.sensors_history.add(secs, &self.stat.sensors);
        self.history.fdinfo_history.add(secs, self.stat.fdinfo.fold_fdinfo_usage());
    }
}

pub struct MyApp {
    pub device_list: Vec<DeviceListMenu>,
    pub fdinfo_sort: FdInfoSortType,
    pub reverse_sort: bool,
    pub vec_device_info: Vec<AppDeviceInfo>,
    pub device_info: AppDeviceInfo,
    pub buf_data: GuiAppData,
    pub arc_data: Arc<Mutex<Vec<GuiAppData>>>,
    pub show_sidepanel: bool,
    pub gl_vendor_info: Option<String>,
    pub selected_pci_bus: PCI::BUS_INFO,
    pub no_pc: bool,
    pub pause: bool,
}

pub fn grid(ui: &mut egui::Ui, v: &[(&str, &str)]) {
    for (name, val) in v {
        ui.label(*name);
        ui.label(*val);
        ui.end_row();
    }
}

pub trait AvgActivity {
    fn avg_activity(&self, ui: &mut egui::Ui);
}

impl AvgActivity for GpuMetrics {
    fn avg_activity(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("{} =>", fl!("avg_activity")));
            let activity = stat::GpuActivity::from_gpu_metrics(self);

            for (val, label) in [
                (activity.gfx, fl!("gfx")),
                (activity.umc, fl!("memory")),
                (activity.media, fl!("media")),
            ] {
                if let Some(val) = val {
                    ui.label(format!("{label} {val:>3}%,"));
                } else {
                    ui.label(format!("{label} ___%,"));
                }
            }
        });
    }
}

pub trait GuiMemoryErrorCount {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiMemoryErrorCount for RasErrorCount {
    fn ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("ECC Memory Error Count").show(ui, |ui| {
            ui.label(fl!("corrected"));
            ui.label(self.corrected.to_string());
            ui.end_row();

            ui.label(fl!("uncorrected"));
            ui.label(self.uncorrected.to_string());
            ui.end_row();
        });
    }
}

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
        self.avg_activity(ui);

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
        self.avg_activity(ui);

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
                ui.label(format!("GFXCLK (Current) =>"));
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

        self.socket_power(ui);
        self.avg_activity(ui);

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

impl MyApp {
    pub fn egui_perf_counter(
        &self,
        ui: &mut egui::Ui,
        pc_name: &str,
        pc: &PerfCounter,
        history: &[History<u8>],
    ) {
        let label_fmt = |_s: &str, val: &PlotPoint| {
            format!("{:.1}s : {:.0}%", val.x, val.y)
        };
        let mut n = 1;

        egui::Grid::new(pc_name).spacing(SPACING).show(ui, |ui| {
            for ((name, pos), history) in pc.index.iter().zip(history.iter()) {
                egui::Grid::new(name).show(ui, |ui| {
                    let usage = pc.bits.get(*pos);
                    ui.label(format!("{name} {usage:3}%"));
                    ui.end_row();

                    let points: PlotPoints = history.iter()
                        .map(|(i, val)| [i, val as f64]).collect();
                    let line = Line::new(points).fill(1.0);

                    default_plot(name)
                        .allow_scroll(false)
                        .include_y(0.0)
                        .include_y(100.0)
                        .label_formatter(label_fmt)
                        .auto_bounds([true, false].into())
                        .height(SENSORS_HEIGHT / 2.0)
                        .width(SENSORS_WIDTH)
                        .show(ui, |plot_ui| plot_ui.line(line));
                });

                n += 1;
                if n % 2 == 1 { ui.end_row(); }
            }
        });
    }

    pub fn egui_vram_plot(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} MiB", val.x, val.y)
        };

        let [vram, gtt] = [
            &self.buf_data.history.vram_history,
            &self.buf_data.history.gtt_history,
        ].map(|history| {
            history.iter().map(|(i, usage)| [i, (usage >> 20) as f64]).collect()
        });

        let max = std::cmp::max(
          self.buf_data.stat.vram_usage.0.vram.total_heap_size >> 20,
          self.buf_data.stat.vram_usage.0.gtt.total_heap_size >> 20,
        );

        default_plot("VRAM Plot")
            .allow_scroll(false)
            .include_y(max as f64)
            .label_formatter(label_fmt)
            .auto_bounds([true, false].into())
            .height(ui.available_width() / 4.0)
            .width(ui.available_width() - 36.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (usage, name) in [
                    (vram, fl!("vram")),
                    (gtt, fl!("gtt"))
                ] {
                    plot_ui.line(Line::new(PlotPoints::new(usage)).name(name));
                }
            });
    }

    pub fn egui_vram(&self, ui: &mut egui::Ui) {
        collapsing_plot(ui, &fl!("vram_plot"), true, |ui| self.egui_vram_plot(ui));

        egui::Grid::new("VRAM").show(ui, |ui| {
            let mib = fl!("mib");
            for (v, name) in [
                (&self.buf_data.stat.vram_usage.0.vram, fl!("vram")),
                (&self.buf_data.stat.vram_usage.0.cpu_accessible_vram, fl!("cpu_visible_vram")),
                (&self.buf_data.stat.vram_usage.0.gtt, fl!("gtt")),
            ] {
                let progress = (v.heap_usage >> 20) as f32 / (v.total_heap_size >> 20) as f32;
                let text = format!("{:5} / {:5} ({}: {:5}) {mib}", v.heap_usage >> 20, v.total_heap_size >> 20, fl!("usable"), v.usable_heap_size >> 20);
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

    pub fn egui_fdinfo_plot(&self, ui: &mut egui::Ui, has_vcn_unified: bool, has_vpe: bool) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0}%", val.x, val.y)
        };

        let [mut gfx, mut compute, mut dma, mut dec, mut enc, mut media, mut vpe] = [0; 7]
            .map(|_| Vec::<[f64; 2]>::with_capacity(HISTORY_LENGTH.end));

        for (i, usage) in self.buf_data.history.fdinfo_history.iter() {
            gfx.push([i, usage.gfx as f64]);
            compute.push([i, usage.compute as f64]);
            dma.push([i, usage.dma as f64]);

            if has_vcn_unified {
                media.push([i, usage.media as f64]);
            } else {
                dec.push([i, usage.total_dec as f64]);
                enc.push([i, usage.total_enc as f64]);
            }

            if has_vpe {
                vpe.push([i, usage.vpe as f64]);
            }
        }

        default_plot("fdinfo Plot")
            .allow_scroll(false)
            .include_y(100.0)
            .label_formatter(label_fmt)
            .auto_bounds([true, false].into())
            .height(ui.available_width() / 4.0)
            .width(ui.available_width() - 36.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (usage, name) in [
                    (gfx, fl!("gfx")),
                    (compute, fl!("compute")),
                    (dma, fl!("dma")),
                ] {
                    plot_ui.line(Line::new(PlotPoints::new(usage)).name(name));
                }

                if has_vcn_unified {
                    plot_ui.line(Line::new(PlotPoints::new(media)).name(fl!("media")));
                } else {
                    plot_ui.line(Line::new(PlotPoints::new(dec)).name(fl!("decode")));
                    plot_ui.line(Line::new(PlotPoints::new(enc)).name(fl!("encode")));
                }

                if has_vpe {
                    plot_ui.line(Line::new(PlotPoints::new(vpe)).name(fl!("vpe")));
                }
            });
    }

    pub fn egui_grid_fdinfo(&mut self, ui: &mut egui::Ui) {
        let has_vcn_unified = self.buf_data.stat.fdinfo.has_vcn_unified;
        let has_vpe = self.buf_data.stat.fdinfo.has_vpe;

        collapsing_plot(
            ui,
            &fl!("fdinfo_plot"),
            true,
            |ui| self.egui_fdinfo_plot(ui, has_vcn_unified, has_vpe),
        );

        egui::Grid::new("fdinfo").show(ui, |ui| {
            ui.style_mut().override_font_id = Some(MEDIUM);
            ui.label(rt_base(format!("{:^15}", fl!("name")))).highlight();
            ui.label(rt_base(format!("{:^8}", fl!("pid")))).highlight();
            ui.label("KFD").highlight();

            for (s, align, sort_type, flag) in [
                (fl!("vram"), 10, FdInfoSortType::VRAM, true),
                (fl!("gtt"), 10, FdInfoSortType::GTT, true),
                (fl!("cpu"), 5, FdInfoSortType::CPU, true),
                (fl!("gfx"), 5, FdInfoSortType::GFX, true),
                (fl!("compute"), 9, FdInfoSortType::Compute, true),
                (fl!("dma"), 5, FdInfoSortType::DMA, true),
                (fl!("media"), 5, FdInfoSortType::MediaEngine, has_vcn_unified),
                (fl!("decode"), 9, FdInfoSortType::Decode, !has_vcn_unified),
                (fl!("encode"), 9, FdInfoSortType::Encode, !has_vcn_unified),
                (fl!("vpe"), 5, FdInfoSortType::VPE, has_vpe),
            ] {
                if !flag { continue; }

                let mark = match (self.fdinfo_sort == sort_type, self.reverse_sort) {
                    (true, false) => "▽ ",
                    (true, true) => "△ ",
                    _ => "",
                };
                let s = format!("{mark}{s}");
                let s = format!("{s:^align$}");
                if ui.button(rt_base(s)).clicked() {
                    self.set_fdinfo_sort_type(sort_type);
                }
            }

            ui.end_row();

            self.buf_data.stat.fdinfo.sort_proc_usage(self.fdinfo_sort, self.reverse_sort);

            let mib = fl!("mib");

            for pu in &self.buf_data.stat.fdinfo.proc_usage {
                ui.label(pu.name.to_string());
                ui.label(format!("{:>8}", pu.pid));
                ui.label(if pu.is_kfd_process { " Y " } else { "" });
                ui.label(format!("{:5} {mib}", pu.usage.vram_usage >> 10));
                ui.label(format!("{:5} {mib}", pu.usage.gtt_usage >> 10));
                for usage in [
                    pu.cpu_usage,
                    pu.usage.gfx,
                    pu.usage.compute,
                    pu.usage.dma,
                ] {
                    ui.label(format!("{usage:3} %"));
                }

                if has_vcn_unified {
                    ui.label(format!("{:3} %", pu.usage.media));
                } else {
                    ui.label(format!("{:3} %", pu.usage.total_dec));
                    ui.label(format!("{:3} %", pu.usage.total_enc));
                }

                if has_vpe {
                    ui.label(format!("{:3} %", pu.usage.vpe));
                }

                ui.end_row();
            } // proc_usage
        });
    }

    pub fn egui_sensors(&self, ui: &mut egui::Ui) {
        ui.style_mut().override_font_id = Some(MEDIUM);
        let sensors = &self.buf_data.stat.sensors;
        let mut n = 1;

        egui::Grid::new("Sensors").spacing(SPACING).show(ui, |ui| {
            for (history, val, label, min, max, unit) in [
                (
                    &self.buf_data.history.sensors_history.sclk,
                    sensors.sclk,
                    "GFX_SCLK",
                    // some AMD GPUs support DS (Deep Sleep) state
                    0,
                    self.device_info.max_gpu_clk,
                    fl!("mhz"),
                ),
                (
                    &self.buf_data.history.sensors_history.mclk,
                    sensors.mclk,
                    "GFX_MCLK",
                    // some AMD GPUs support DS (Deep Sleep) state
                    0,
                    self.device_info.max_mem_clk,
                    fl!("mhz"),
                ),
                (
                    &self.buf_data.history.sensors_history.vddgfx,
                    sensors.vddgfx,
                    "VDDGFX",
                    500, // "500 mV" is not an exact value
                    1500, // "1500 mV" is not an exact value
                    fl!("mv"),
                ),
                (
                    &self.buf_data.history.sensors_history.average_power,
                    sensors.average_power.as_ref().map(|power| power.value),
                    "Average Power",
                    0,
                    if let Some(cap) = &sensors.power_cap { cap.current } else { 350 }, // "350 W" is not an exact value
                    fl!("w"),
                ),
                (
                    &self.buf_data.history.sensors_history.input_power,
                    sensors.input_power.as_ref().map(|power| power.value),
                    "Input Power",
                    0,
                    if let Some(cap) = &sensors.power_cap { cap.current } else { 350 }, // "350 W" is not an exact value
                    fl!("w"),
                ),
                (
                    &self.buf_data.history.sensors_history.fan_rpm,
                    sensors.fan_rpm,
                    "Fan",
                    0,
                    sensors.fan_max_rpm.unwrap_or(6000), // "6000 RPM" is not an exact value
                    fl!("rpm"),
                ),
            ] {
                let Some(val) = val else { continue };

                egui::Grid::new(&label).show(ui, |ui| {
                    ui.label(format!("{label} ({val:4} {unit})"));
                    ui.end_row();

                    let label_fmt = move |_name: &str, val: &PlotPoint| {
                        format!("{:.1}s\n{:.0} {unit}", val.x, val.y)
                    };
                    let points: PlotPoints = history.iter()
                        .map(|(i, val)| [i, val as f64]).collect();
                    let line = Line::new(points).fill(1.0);

                    Plot::new(label)
                        .allow_zoom(false)
                        .allow_scroll(false)
                        .include_y(min)
                        .include_y(max)
                        .show_axes(false)
                        .label_formatter(label_fmt)
                        .auto_bounds([true, false].into())
                        .height(SENSORS_HEIGHT)
                        .width(SENSORS_WIDTH)
                        .show(ui, |plot_ui| plot_ui.line(line));
                });

                n += 1;
                if n % 2 == 1 { ui.end_row(); }
            }

            if n % 2 == 0 { ui.end_row(); }
            self.egui_temp_plot(ui);
        });

        if let Some(cur) = sensors.current_link {
            let min_max = if let [Some(min), Some(max)] = [sensors.min_dpm_link, sensors.max_dpm_link] {
                format!(
                    " (Gen{}x{} - Gen{}x{})",
                    min.gen,
                    min.width,
                    max.gen,
                    max.width,
                )
            } else if let Some(max) = sensors.max_dpm_link {
                format!(" ({} Gen{}x{})", fl!("max"), max.gen, max.width)
            } else {
                String::new()
            };

            ui.label(format!(
                "{} => Gen{}x{} {min_max}",
                fl!("pcie_link_speed"),
                cur.gen,
                cur.width,
            ));
        }

        if let Some(power_state) = &sensors.pci_power_state {
            ui.label(format!(
                "{}: {}",
                fl!("pci_power_state"),
                power_state,
            ));
        }
    }

    pub fn egui_temp_plot(&self, ui: &mut egui::Ui) {
        ui.style_mut().override_font_id = Some(MEDIUM);
        let sensors = &self.buf_data.stat.sensors;
        let label_fmt = |_name: &str, val: &PlotPoint| {
            format!("{:.1}s\n{:.0} C", val.x, val.y)
        };
        let mut n = 1;

        for (label, temp, temp_history) in [
            ("Edge", &sensors.edge_temp, &self.buf_data.history.sensors_history.edge_temp),
            ("Junction", &sensors.junction_temp, &self.buf_data.history.sensors_history.junction_temp),
            ("Memory", &sensors.memory_temp, &self.buf_data.history.sensors_history.memory_temp),
        ] {
            let Some(temp) = temp else { continue };

            egui::Grid::new(&label).show(ui, |ui| {
                let val = temp.current;
                let max = temp.critical.unwrap_or(105) as f64;

                ui.label(format!("{label} Temp. ({val:4} C)"));
                ui.end_row();

                let points: PlotPoints = temp_history.iter()
                    .map(|(i, val)| [i, val as f64]).collect();
                let line = Line::new(points).fill(1.0);

                default_plot(label)
                    .include_y(max)
                    .label_formatter(label_fmt)
                    .auto_bounds([true, true].into())
                    .height(SENSORS_HEIGHT)
                    .width(SENSORS_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
            });

            n += 1;
            if n % 2 == 1 { ui.end_row(); }
        }
    }

    pub fn egui_pcie_bw(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} {}", val.x, val.y, fl!("mib_s"))
        };

        let fl_sent = fl!("sent");
        let fl_rec = fl!("received");
        let mib_s = fl!("mib_s");

        let [sent, rec] = {
            let [mut sent_history, mut rec_history] = [0; 2].map(|_| Vec::<[f64; 2]>::new());

            for (i, (sent, rec)) in self.buf_data.history.pcie_bw_history.iter() {
                sent_history.push([i, sent as f64]);
                rec_history.push([i, rec as f64]);
            }

            [
                Line::new(PlotPoints::new(sent_history)).name(&fl_sent),
                Line::new(PlotPoints::new(rec_history)).name(&fl_rec),
            ]
        };

        default_plot("pcie_bw plot")
            .label_formatter(label_fmt)
            .auto_bounds([true, true].into())
            .height(ui.available_width() / 4.0)
            .width(ui.available_width() - 36.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(sent);
                plot_ui.line(rec);
            });

        if let Some((sent, rec)) = self.buf_data.history.pcie_bw_history.latest() {
            ui.label(format!("{fl_sent}: {sent:5} {mib_s}, {fl_rec}: {rec:5} {mib_s}"));
        } else {
            ui.label(format!("{fl_sent}: _ {mib_s}, {fl_rec}: _ {mib_s}"));
        }
    }
}

fn default_plot(id: &str) -> Plot {
    Plot::new(id)
        .allow_zoom(false)
        .allow_scroll(false)
        .include_y(0.0)
        .show_axes(false)
}
