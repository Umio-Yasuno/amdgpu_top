use crate::egui::util::History;
use egui_plot::PlotPoint;
use crate::HISTORY_LENGTH;

use libamdgpu_top::{AppDeviceInfo, ConnectorInfo, DevicePath, PCI};
use libamdgpu_top::app::{
    AppAmdgpuTop,
    AppAmdgpuTopStat,
};
use libamdgpu_top::AMDGPU::{MetricsInfo, ThrottleStatus};
use libamdgpu_top::stat::{
    FdInfoUsage,
    Sensors,
    gpu_metrics_util,
};

#[derive(Debug, Clone)]
pub struct PlotHistory<T> {
    pub value_history: History<T>,
    pub vec_plotpoint: Vec<PlotPoint>,
}

impl<T: std::marker::Copy> PlotHistory<T> {
    pub fn new() -> Self {
        Self {
            value_history: History::new(HISTORY_LENGTH, f32::INFINITY),
            vec_plotpoint: Vec::new(),
        }
    }

    pub fn add(&mut self, now: f64, value: T) {
        self.value_history.add(now, value);
    }

    pub fn add_and_update<F>(&mut self, now: f64, value: T, data_func: F)
    where
        F: Fn(T) -> f64
    {
        self.add(now, value);
        self.vec_plotpoint.clear();
        self.vec_plotpoint.extend(
            self.value_history
                .iter()
                .map(|(i, v)| PlotPoint::from([i, data_func(v)]))
        );
    }
}

#[derive(Clone)]
pub struct HistoryData {
    pub grbm_history: Vec<PlotHistory<u8>>,
    pub grbm2_history: Vec<PlotHistory<u8>>,
    pub vram_history: PlotHistory<u64>,
    pub gtt_history: PlotHistory<u64>,
    pub fdinfo_history: History<FdInfoUsage>,
    pub gfx_plot: Vec<PlotPoint>,
    pub compute_plot: Vec<PlotPoint>,
    pub dma_plot: Vec<PlotPoint>,
    pub dec_plot: Vec<PlotPoint>,
    pub enc_plot: Vec<PlotPoint>,
    pub vcnu_plot: Vec<PlotPoint>,
    pub vpe_plot: Vec<PlotPoint>,
    pub sensors_history: SensorsHistory,
    pub pcie_sent_bw_history: PlotHistory<u64>,
    pub pcie_rec_bw_history: PlotHistory<u64>,
    pub throttling_history: History<ThrottleStatus>,
    pub gfx_activity: PlotHistory<u16>,
    pub umc_activity: PlotHistory<u16>,
    pub media_activity: PlotHistory<u16>,
    pub avg_vclk: PlotHistory<u16>,
    pub avg_dclk: PlotHistory<u16>,
    pub avg_vclk1: PlotHistory<u16>,
    pub avg_dclk1: PlotHistory<u16>,
    pub cur_vclk: PlotHistory<u16>,
    pub cur_dclk: PlotHistory<u16>,
    pub cur_vclk1: PlotHistory<u16>,
    pub cur_dclk1: PlotHistory<u16>,
    pub core_temp: Option<Vec<PlotHistory<u16>>>,
    pub core_power_mw: Option<Vec<PlotHistory<u16>>>,
}

#[derive(Debug, Clone)]
pub struct SensorsHistory {
    pub sclk: PlotHistory<u32>,
    pub mclk: PlotHistory<u32>,
    pub fclk: PlotHistory<u32>,
    pub vddgfx: PlotHistory<u32>,
    pub vddnb: PlotHistory<u32>,
    pub edge_temp: PlotHistory<i64>,
    pub junction_temp: PlotHistory<i64>,
    pub memory_temp: PlotHistory<i64>,
    pub average_power: PlotHistory<u32>,
    pub input_power: PlotHistory<u32>,
    pub fan_rpm: PlotHistory<u32>,
    pub tctl: PlotHistory<i64>,
    pub core_freq: Vec<PlotHistory<u32>>,
}

impl SensorsHistory {
    pub fn new() -> Self {
        let [sclk, mclk, fclk, vddgfx, vddnb, average_power, input_power, fan_rpm] = [0; 8]
            .map(|_| PlotHistory::new());
        let [edge_temp, junction_temp, memory_temp, tctl] = [0;4]
            .map(|_| PlotHistory::new());
        let core_freq = vec![PlotHistory::new(); 64];

        Self { sclk, mclk, fclk, vddgfx, vddnb, edge_temp, junction_temp, memory_temp, average_power, input_power, fan_rpm, tctl, core_freq }
    }

    pub fn add(&mut self, sec: f64, sensors: &Sensors) {
        for (history, val) in [
            (&mut self.sclk, sensors.sclk),
            (&mut self.mclk, sensors.mclk),
            (&mut self.fclk, sensors.fclk_dpm.as_ref().map(|f| f.current_mhz)),
            (&mut self.vddgfx, sensors.vddgfx),
            (&mut self.vddnb, sensors.vddnb),
            (&mut self.average_power, sensors.average_power.as_ref().map(|power| power.value)),
            (&mut self.input_power, sensors.input_power.as_ref().map(|power| power.value)),
            (&mut self.fan_rpm, sensors.fan_rpm),
        ] {
            let Some(val) = val else { continue };
            history.add_and_update(sec, val, |v| v as f64);
        }

        for (history, temp) in [
            (&mut self.edge_temp, &sensors.edge_temp),
            (&mut self.junction_temp, &sensors.junction_temp),
            (&mut self.memory_temp, &sensors.memory_temp),
        ] {
            let Some(temp) = temp else { continue };
            history.add_and_update(sec, temp.current, |v| v as f64);
        }

        if let Some(tctl_val) = sensors.tctl {
            self.tctl.add_and_update(sec, tctl_val / 1000, |v| v as f64);
        }

        for (freq, freq_history) in sensors.all_cpu_core_freq_info.iter().zip(self.core_freq.iter_mut()) {
            freq_history.add_and_update(sec, freq.cur, |v| v as f64);
        }
    }
}

impl Default for SensorsHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GuiAppData {
    pub stat: AppAmdgpuTopStat,
    pub device_info: AppDeviceInfo,
    pub pci_bus: PCI::BUS_INFO,
    pub support_pcie_bw: bool,
    pub history: HistoryData,
    pub vec_connector_info: Vec<ConnectorInfo>,
    pub xdna_device_path: Option<DevicePath>,
    pub xdna_fw_version: Option<String>,
}

impl GuiAppData {
    pub fn new(app: &AppAmdgpuTop) -> Self {
        let vram_history = PlotHistory::new();
        let gtt_history = PlotHistory::new();
        let fdinfo_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let sensors_history = SensorsHistory::default();
        let pcie_sent_bw_history = PlotHistory::new();
        let pcie_rec_bw_history = PlotHistory::new();
        let throttling_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let [grbm_history, grbm2_history] = [&app.stat.grbm, &app.stat.grbm2].map(|pc| {
            vec![PlotHistory::new(); pc.pc_index.len()]
        });
        let gfx_activity = PlotHistory::new();
        let umc_activity = PlotHistory::new();
        let media_activity = PlotHistory::new();

        let avg_vclk = PlotHistory::new();
        let avg_dclk = PlotHistory::new();
        let avg_vclk1 = PlotHistory::new();
        let avg_dclk1 = PlotHistory::new();

        let cur_vclk = PlotHistory::new();
        let cur_dclk = PlotHistory::new();
        let cur_vclk1 = PlotHistory::new();
        let cur_dclk1 = PlotHistory::new();

        let checked_core_temp = app.stat.metrics
            .as_ref()
            .and_then(|m| gpu_metrics_util::check_temp_array(m.get_temperature_core()));
        let checked_core_power_mw = app.stat.metrics
            .as_ref()
            .and_then(|m| gpu_metrics_util::check_power_clock_array(m.get_average_core_power()));

        let core_temp =
            checked_core_temp.map(|c| vec![PlotHistory::new(); c.len()]);
        let core_power_mw =
            checked_core_power_mw.map(|p| vec![PlotHistory::new(); p.len()]);

        let xdna_device_path = app.xdna_device_path.clone();
        let xdna_fw_version = app.xdna_fw_version.clone();

        Self {
            stat: app.stat.clone(),
            device_info: app.device_info.clone(),
            pci_bus: app.device_info.pci_bus,
            support_pcie_bw: app.stat.arc_pcie_bw.is_some(),
            history: HistoryData {
                grbm_history,
                grbm2_history,
                vram_history,
                gtt_history,
                fdinfo_history,
                gfx_plot: Vec::new(),
                compute_plot: Vec::new(),
                dma_plot: Vec::new(),
                dec_plot: Vec::new(),
                enc_plot: Vec::new(),
                vcnu_plot: Vec::new(),
                vpe_plot: Vec::new(),
                sensors_history,
                pcie_sent_bw_history,
                pcie_rec_bw_history,
                throttling_history,
                gfx_activity,
                umc_activity,
                media_activity,
                avg_vclk,
                avg_dclk,
                avg_vclk1,
                avg_dclk1,
                cur_vclk,
                cur_dclk,
                cur_vclk1,
                cur_dclk1,
                core_temp,
                core_power_mw,
            },
            vec_connector_info: libamdgpu_top::connector_info(&app.device_path),
            xdna_device_path,
            xdna_fw_version,
        }
    }

    pub fn update_history(&mut self, secs: f64, no_pc: bool) {
        let metrics = self.stat.metrics.as_ref();

        if let Some(arc_pcie_bw) = &self.stat.arc_pcie_bw {
            let lock = arc_pcie_bw.try_lock();
            if let Ok(pcie_bw) = lock
                && let (Some(sent), Some(rec), Some(mps)) = (
                    pcie_bw.sent,
                    pcie_bw.received,
                    pcie_bw.max_payload_size,
                ) {
                    let sent = (sent * mps as u64) >> 20;
                    let rec = (rec * mps as u64) >> 20;
                    self.history.pcie_sent_bw_history.add_and_update(secs, sent, |v| v as f64);
                    self.history.pcie_rec_bw_history.add_and_update(secs, rec, |v| v as f64);
                }
        }

        if !no_pc {
            for (pc, pc_history) in [
                (&self.stat.grbm, &mut self.history.grbm_history),
                (&self.stat.grbm2, &mut self.history.grbm2_history),
            ] {
                for (pc_index, h) in pc.pc_index.iter().zip(pc_history.iter_mut()) {
                    h.add_and_update(secs, pc_index.usage, |usage| usage as f64);
                }
            }
        }

        if let Some(thr_val) = metrics.and_then(|m| m.get_throttle_status_info())
            && !thr_val.is_zero()
        {
            self.history.throttling_history.add(secs, thr_val);
        }

        if let Some(ref mut sensors) = self.stat.sensors {
            self.history.sensors_history.add(secs, sensors);
        }

        for (mem_history, value) in [
            (&mut self.history.vram_history, self.stat.vram_usage.0.vram.heap_usage),
            (&mut self.history.gtt_history, self.stat.vram_usage.0.gtt.heap_usage),
        ] {
            mem_history.add_and_update(secs, value, |usage| (usage >> 20) as f64);
        }

        self.history.fdinfo_history.add(secs, self.stat.fdinfo.fold_fdinfo_usage().0);

        for plot in [
            &mut self.history.gfx_plot,
            &mut self.history.compute_plot,
            &mut self.history.dma_plot,
            &mut self.history.dec_plot,
            &mut self.history.enc_plot,
            &mut self.history.vcnu_plot,
            &mut self.history.vpe_plot,
        ] {
            plot.clear();
        }

        for (i, usage) in self.history.fdinfo_history.iter() {
            for (plot, engine_usage) in [
                (&mut self.history.gfx_plot, usage.gfx),
                (&mut self.history.compute_plot, usage.compute),
                (&mut self.history.dma_plot, usage.dma),
                (&mut self.history.dec_plot, usage.dec),
                (&mut self.history.enc_plot, usage.enc),
                (&mut self.history.vcnu_plot, usage.vcn_unified),
                (&mut self.history.vpe_plot, usage.vpe),
            ] {
                plot.push(PlotPoint::from([i, engine_usage as f64]));
            }
        }

        for (activity_history, value) in [
            (&mut self.history.gfx_activity, self.stat.activity.gfx),
            (&mut self.history.umc_activity, self.stat.activity.umc),
            (&mut self.history.media_activity, self.stat.activity.media),
        ] {
            let Some(value) = value else { continue };
            activity_history.add_and_update(secs, value, |per| per as f64)
        }

        for (clk_history, val) in [
            (&mut self.history.avg_vclk, metrics.and_then(|m| m.get_average_vclk_frequency())),
            (&mut self.history.avg_dclk, metrics.and_then(|m| m.get_average_dclk_frequency())),
            (&mut self.history.avg_vclk1, metrics.and_then(|m| m.get_average_vclk1_frequency())),
            (&mut self.history.avg_dclk1, metrics.and_then(|m| m.get_average_dclk1_frequency())),
            (&mut self.history.cur_vclk, metrics.and_then(|m| m.get_current_vclk())),
            (&mut self.history.cur_dclk, metrics.and_then(|m| m.get_current_dclk())),
            (&mut self.history.cur_vclk1, metrics.and_then(|m| m.get_current_vclk1())),
            (&mut self.history.cur_dclk1, metrics.and_then(|m| m.get_current_dclk1())),
        ] {
            let Some(val) = val else { continue };
            if val == u16::MAX { continue }

            clk_history.add_and_update(secs, val, |clk| clk as f64);
        }

        if let Some(ref mut history_core_temp) = self.history.core_temp
            && let Some(core_temp) = metrics
                .and_then(|m| gpu_metrics_util::check_temp_array(m.get_temperature_core()))
            {
                for (history, temp) in history_core_temp.iter_mut().zip(core_temp.iter()) {
                    history.add_and_update(secs, *temp, |v| v as f64);
                }
            }

        if let Some(ref mut history_core_power_mw) = self.history.core_power_mw
            && let Some(core_power_mw) = metrics
                .and_then(|m| gpu_metrics_util::check_power_clock_array(m.get_average_core_power()))
            {
                for (history, power) in history_core_power_mw.iter_mut().zip(core_power_mw.iter()) {
                    history.add_and_update(secs, *power, |v| v as f64);
                }
            }
    }
}
