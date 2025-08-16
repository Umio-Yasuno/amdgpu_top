use crate::egui::util::History;
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

#[derive(Clone)]
pub struct HistoryData {
    pub grbm_history: Vec<History<u8>>,
    pub grbm2_history: Vec<History<u8>>,
    pub vram_history: History<u64>,
    pub gtt_history: History<u64>,
    pub fdinfo_history: History<FdInfoUsage>,
    pub sensors_history: SensorsHistory,
    pub pcie_bw_history: History<(u64, u64)>,
    pub throttling_history: History<ThrottleStatus>,
    pub gfx_activity: History<u16>,
    pub umc_activity: History<u16>,
    pub media_activity: History<u16>,
    pub vclk: History<u16>,
    pub dclk: History<u16>,
    pub vclk1: History<u16>,
    pub dclk1: History<u16>,
    pub core_temp: Option<Vec<History<u16>>>,
    pub core_power_mw: Option<Vec<History<u16>>>,
}

#[derive(Debug, Clone)]
pub struct SensorsHistory {
    pub sclk: History<u32>,
    pub mclk: History<u32>,
    pub fclk: History<u32>,
    pub vddgfx: History<u32>,
    pub vddnb: History<u32>,
    pub edge_temp: History<i64>,
    pub junction_temp: History<i64>,
    pub memory_temp: History<i64>,
    pub average_power: History<u32>,
    pub input_power: History<u32>,
    pub fan_rpm: History<u32>,
    pub tctl: History<i64>,
    pub core_freq: Vec<History<u32>>,
}

impl SensorsHistory {
    pub fn new() -> Self {
        let [sclk, mclk, fclk, vddgfx, vddnb, average_power, input_power, fan_rpm] = [0; 8]
            .map(|_| History::new(HISTORY_LENGTH, f32::INFINITY));
        let [edge_temp, junction_temp, memory_temp, tctl] = [0;4]
            .map(|_| History::new(HISTORY_LENGTH, f32::INFINITY));
        let core_freq = vec![History::new(HISTORY_LENGTH, f32::INFINITY); 64];

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
            history.add(sec, val);
        }

        for (history, temp) in [
            (&mut self.edge_temp, &sensors.edge_temp),
            (&mut self.junction_temp, &sensors.junction_temp),
            (&mut self.memory_temp, &sensors.memory_temp),
        ] {
            let Some(temp) = temp else { continue };
            history.add(sec, temp.current);
        }

        if let Some(tctl_val) = sensors.tctl {
            self.tctl.add(sec, tctl_val / 1000);
        }

        for (freq, freq_history) in sensors.all_cpu_core_freq_info.iter().zip(self.core_freq.iter_mut()) {
            freq_history.add(sec, freq.cur);
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
        let vram_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let gtt_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let fdinfo_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let sensors_history = SensorsHistory::default();
        let pcie_bw_history: History<(u64, u64)> = History::new(HISTORY_LENGTH, f32::INFINITY);
        let throttling_history = History::new(HISTORY_LENGTH, f32::INFINITY);
        let [grbm_history, grbm2_history] = [&app.stat.grbm, &app.stat.grbm2].map(|pc| {
            vec![History::<u8>::new(HISTORY_LENGTH, f32::INFINITY); pc.pc_index.len()]
        });
        let gfx_activity = History::new(HISTORY_LENGTH, f32::INFINITY);
        let umc_activity = History::new(HISTORY_LENGTH, f32::INFINITY);
        let media_activity = History::new(HISTORY_LENGTH, f32::INFINITY);

        let vclk = History::new(HISTORY_LENGTH, f32::INFINITY);
        let dclk = History::new(HISTORY_LENGTH, f32::INFINITY);
        let vclk1 = History::new(HISTORY_LENGTH, f32::INFINITY);
        let dclk1 = History::new(HISTORY_LENGTH, f32::INFINITY);

        let checked_core_temp = app.stat.metrics
            .as_ref()
            .and_then(|m| gpu_metrics_util::check_temp_array(m.get_temperature_core()));
        let checked_core_power_mw = app.stat.metrics
            .as_ref()
            .and_then(|m| gpu_metrics_util::check_power_clock_array(m.get_average_core_power()));

        let core_temp =
            checked_core_temp.map(|c| vec![History::<u16>::new(HISTORY_LENGTH, f32::INFINITY); c.len()]);
        let core_power_mw =
            checked_core_power_mw.map(|p| vec![History::<u16>::new(HISTORY_LENGTH, f32::INFINITY); p.len()]);

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
                sensors_history,
                pcie_bw_history,
                throttling_history,
                gfx_activity,
                umc_activity,
                media_activity,
                vclk,
                dclk,
                vclk1,
                dclk1,
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
                    self.history.pcie_bw_history.add(secs, (sent, rec));
                }
        }

        if !no_pc {
            for (pc, history) in [
                (&self.stat.grbm, &mut self.history.grbm_history),
                (&self.stat.grbm2, &mut self.history.grbm2_history),
            ] {
                for (pc_index, h) in pc.pc_index.iter().zip(history.iter_mut()) {
                    h.add(secs, pc_index.usage);
                }
            }
        }

        if let Some(thr_val) = metrics.and_then(|m| m.get_indep_throttle_status())
            && thr_val != 0
        {
            self.history.throttling_history.add(secs, ThrottleStatus::new(thr_val));
        }

        if let Some(ref mut sensors) = self.stat.sensors {
            self.history.sensors_history.add(secs, sensors);
        }

        self.history.vram_history.add(secs, self.stat.vram_usage.0.vram.heap_usage);
        self.history.gtt_history.add(secs, self.stat.vram_usage.0.gtt.heap_usage);
        self.history.fdinfo_history.add(secs, self.stat.fdinfo.fold_fdinfo_usage().0);

        if let Some(gfx) = self.stat.activity.gfx {
            self.history.gfx_activity.add(secs, gfx);
        }
        if let Some(umc) = self.stat.activity.umc {
            self.history.umc_activity.add(secs, umc);
        }
        if let Some(media) = self.stat.activity.media {
            self.history.media_activity.add(secs, media);
        }

        if let Some(vclk) = metrics.and_then(|m| m.get_current_vclk()) {
            self.history.vclk.add(secs, vclk);
        }
        if let Some(dclk) = metrics.and_then(|m| m.get_current_dclk()) {
            self.history.dclk.add(secs, dclk);
        }
        if let Some(vclk1) = metrics.and_then(|m| m.get_current_vclk1()) {
            self.history.vclk1.add(secs, vclk1);
        }
        if let Some(dclk1) = metrics.and_then(|m| m.get_current_dclk1()) {
            self.history.dclk1.add(secs, dclk1);
        }

        if let Some(ref mut history_core_temp) = self.history.core_temp
            && let Some(core_temp) = metrics
                .and_then(|m| gpu_metrics_util::check_temp_array(m.get_temperature_core()))
            {
                for (history, v) in history_core_temp.iter_mut().zip(core_temp.iter()) {
                    history.add(secs, *v);
                }
            }

        if let Some(ref mut history_core_power_mw) = self.history.core_power_mw
            && let Some(core_power_mw) = metrics
                .and_then(|m| gpu_metrics_util::check_power_clock_array(m.get_average_core_power()))
            {
                for (history, v) in history_core_power_mw.iter_mut().zip(core_power_mw.iter()) {
                    history.add(secs, *v);
                }
            }
    }
}
