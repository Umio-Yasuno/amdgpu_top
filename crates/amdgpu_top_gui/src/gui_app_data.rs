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
    pub core_power_mw: Option<Vec<History<u16>>>,
}

#[derive(Debug, Clone)]
pub struct SensorsHistory {
    pub sclk: History<u32>,
    pub mclk: History<u32>,
    pub vddgfx: History<u32>,
    pub vddnb: History<u32>,
    pub edge_temp: History<i64>,
    pub junction_temp: History<i64>,
    pub memory_temp: History<i64>,
    pub average_power: History<u32>,
    pub input_power: History<u32>,
    pub fan_rpm: History<u32>,
}

impl SensorsHistory {
    pub fn new() -> Self {
        let [sclk, mclk, vddgfx, vddnb, average_power, input_power, fan_rpm] = [0; 7]
            .map(|_| History::new(HISTORY_LENGTH, f32::INFINITY));
        let [edge_temp, junction_temp, memory_temp] = [0;3]
            .map(|_| History::new(HISTORY_LENGTH, f32::INFINITY));

        Self { sclk, mclk, vddgfx, vddnb, edge_temp, junction_temp, memory_temp, average_power, input_power, fan_rpm }
    }

    pub fn add(&mut self, sec: f64, sensors: &Sensors) {
        for (history, val) in [
            (&mut self.sclk, sensors.sclk),
            (&mut self.mclk, sensors.mclk),
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

        let checked_core_power_mw = app.stat.metrics
            .as_ref()
            .and_then(|m| gpu_metrics_util::check_power_clock_array(m.get_average_core_power()));

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
                core_power_mw,
            },
            vec_connector_info: libamdgpu_top::connector_info(&app.device_path),
            xdna_device_path,
            xdna_fw_version,
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
                for (pc_index, h) in pc.pc_index.iter().zip(history.iter_mut()) {
                    h.add(secs, pc_index.usage);
                }
            }
        }

        if let Some(thr_val) = self.stat.metrics.as_ref().and_then(|m| m.get_indep_throttle_status()) {
            if thr_val != 0 {
                self.history.throttling_history.add(secs, ThrottleStatus::new(thr_val));
            }
        }

        if let Some(ref mut sensors) = self.stat.sensors {
            self.history.sensors_history.add(secs, sensors);
        }

        self.history.vram_history.add(secs, self.stat.vram_usage.0.vram.heap_usage);
        self.history.gtt_history.add(secs, self.stat.vram_usage.0.gtt.heap_usage);
        self.history.fdinfo_history.add(secs, self.stat.fdinfo.fold_fdinfo_usage());

        if let Some(gfx) = self.stat.activity.gfx {
            self.history.gfx_activity.add(secs, gfx);
        }
        if let Some(umc) = self.stat.activity.umc {
            self.history.umc_activity.add(secs, umc);
        }
        if let Some(media) = self.stat.activity.media {
            self.history.media_activity.add(secs, media);
        }

        if let Some(ref mut history_core_power_mw) = self.history.core_power_mw {
            if let Some(core_power_mw) = self.stat.metrics
                .as_ref()
                .and_then(|m| gpu_metrics_util::check_power_clock_array(m.get_average_core_power()))
            {
                for (history, v) in history_core_power_mw.iter_mut().zip(core_power_mw.iter()) {
                    history.add(secs, *v);
                }
            }
        }
    }
}
