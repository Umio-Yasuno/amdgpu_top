use std::fmt::{self, Write};
use super::{Text, Opt};
use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, GpuMetrics, MetricsInfo};
use std::path::PathBuf;

const CORE_TEMP_LABEL: &str = "Core Temp (C)";
const CORE_POWER_LABEL: &str = "Core Power (mW)";
const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

pub struct GpuMetricsView {
    sysfs_path: PathBuf,
    metrics: GpuMetrics,
    pub text: Text,
}

impl GpuMetricsView {
    pub fn new(amdgpu_dev: &DeviceHandle) -> Self {

        Self {
            sysfs_path: amdgpu_dev.get_sysfs_path().unwrap(),
            metrics: GpuMetrics::Unknown,
            text: Text::default(),
        }
    }

    pub fn update_metrics(&mut self, amdgpu_dev: &DeviceHandle) -> Result<(), ()> {
        if let Ok(metrics) = amdgpu_dev.get_gpu_metrics_from_sysfs_path(&self.sysfs_path) {
            self.metrics = metrics;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn print(&mut self) -> Result<(), fmt::Error> {
        self.text.clear();

        match self.metrics {
            GpuMetrics::V1_0(_) |
            GpuMetrics::V1_1(_) |
            GpuMetrics::V1_2(_) |
            GpuMetrics::V1_3(_) => self.for_v1()?,
            GpuMetrics::V2_0(_) |
            GpuMetrics::V2_1(_) |
            GpuMetrics::V2_2(_) |
            GpuMetrics::V2_3(_) => self.for_v2()?,
            GpuMetrics::Unknown => {},
        };

        Ok(())
    }

    fn for_v1(&mut self) -> Result<(), fmt::Error> {
        if let Some(socket_power) = self.metrics.get_average_socket_power() {
            writeln!(self.text.buf, " Socket Power: {socket_power:3} W")?;
        }

        for (val, name) in [
            (self.metrics.get_temperature_edge(), "Edge"),
            (self.metrics.get_temperature_hotspot(), "Hotspot"),
            (self.metrics.get_temperature_mem(), "Memory"),
            (self.metrics.get_temperature_vrgfx(), "VRGFX"),
            (self.metrics.get_temperature_vrsoc(), "VRSOC"),
            (self.metrics.get_temperature_vrmem(), "VRMEM"),
        ] {
            let Some(v) = val else { continue };
            write!(self.text.buf, " {name}: {v:3} C,")?;
        }
        writeln!(self.text.buf)?;

        for (avg, cur, name) in [
            (
                self.metrics.get_average_gfxclk_frequency(),
                self.metrics.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                self.metrics.get_average_socclk_frequency(),
                self.metrics.get_current_socclk(),
                "SOCCLK",
            ),
            (
                self.metrics.get_average_uclk_frequency(),
                self.metrics.get_current_uclk(),
                "UCLK",
            ),
            (
                self.metrics.get_average_vclk_frequency(),
                self.metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                self.metrics.get_average_dclk_frequency(),
                self.metrics.get_current_dclk(),
                "DCLK",
            ),
            (
                self.metrics.get_average_vclk1_frequency(),
                self.metrics.get_current_vclk1(),
                "VCLK1",
            ),
            (
                self.metrics.get_average_dclk1_frequency(),
                self.metrics.get_current_dclk1(),
                "DCLK1",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(|val| {
                let v = val.unwrap_or(0);

                if v == u16::MAX { 0 } else { v }
            });
            writeln!(self.text.buf, " {name:6} Avg. {avg:4} MHz, Cur. {cur:4} MHz")?;
        }

        for (voltage, name) in [
            (self.metrics.get_voltage_soc(), "SoC"),
            (self.metrics.get_voltage_gfx(), "GFX"),
            (self.metrics.get_voltage_mem(), "Mem"),
        ] {
            let Some(voltage) = voltage else { continue };
            write!(self.text.buf, " {name}: {voltage:4} mV, ")?;
        }
        writeln!(self.text.buf)?;

        if let Some(hbm_temp) = self.metrics.get_temperature_hbm() {
            write!(self.text.buf, "HBM Temp (C) [")?;
            for v in &hbm_temp {
                let v = v.saturating_div(100);
                write!(self.text.buf, "{v:5},")?;
            }
            writeln!(self.text.buf, "]")?;
        }

        Ok(())
    }

    fn for_v2(&mut self) -> Result<(), fmt::Error> {
        write!(self.text.buf, " GFX: ")?;
        for (val, unit, div) in [
            (self.metrics.get_temperature_gfx(), "C", 100),
            (self.metrics.get_average_gfx_power(), "mW", 1),
            (self.metrics.get_current_gfxclk(), "MHz", 1),
        ] {
            let v = val.unwrap_or(0);
            if v == u16::MAX {
                write!(self.text.buf, "{:5} {unit}, ", "")?;
            } else {
                let v = v.saturating_div(div);
                write!(self.text.buf, "{v:5} {unit}, ")?;
            }
        }
        writeln!(self.text.buf)?;

        write!(self.text.buf, " SoC: ")?;
        for (val, unit, div) in [
            (self.metrics.get_temperature_soc(), "C", 100),
            (self.metrics.get_average_soc_power(), "mW", 1),
            (self.metrics.get_current_socclk(), "MHz", 1),
        ] {
            let v = val.unwrap_or(0);
            if v == u16::MAX {
                write!(self.text.buf, "{:5} {unit}, ", "")?;
            } else {
                let v = v.saturating_div(div);
                write!(self.text.buf, "{v:5} {unit}, ")?;
            }
        }
        writeln!(self.text.buf)?;

        if let Some(socket_power) = self.metrics.get_average_socket_power() {
            writeln!(self.text.buf, " Socket Power: {socket_power:3} W")?;
        }
/*
        if let [Some(gfx), Some(mm)] = [
            self.metrics.get_average_gfx_activity(),
            self.metrics.get_average_mm_activity(),
        ] {
            writeln!(self.text.buf, " Activity: {gfx:4} (GFX), {mm:4} (Media)")?;
        }
*/
        for (avg, cur, name) in [
            (
                self.metrics.get_average_uclk_frequency(),
                self.metrics.get_current_uclk(),
                "UCLK",
            ),
            (
                self.metrics.get_average_fclk_frequency(),
                self.metrics.get_current_fclk(),
                "FCLK",
            ),
            (
                self.metrics.get_average_vclk_frequency(),
                self.metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                self.metrics.get_average_dclk_frequency(),
                self.metrics.get_current_dclk(),
                "DCLK",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(|val| {
                let v = val.unwrap_or(0);

                if v == u16::MAX { 0 } else { v }
            });
            writeln!(self.text.buf, " {name} Avg. {avg:4} MHz, Cur. {cur:4} MHz")?;
        }

        for (val, label, div) in [
            (self.metrics.get_temperature_core(), CORE_TEMP_LABEL, 100),
            (self.metrics.get_average_core_power(), CORE_POWER_LABEL, 1),
            (self.metrics.get_current_coreclk(), CORE_CLOCK_LABEL, 1),
        ] {
            let Some(val) = val else { continue };
            write!(self.text.buf, " {label:<16}: [")?;
            for v in &val {
                let v = v.saturating_div(div);
                write!(self.text.buf, "{v:5},")?;
            }
            writeln!(self.text.buf, "]")?;
        }

        for (val, label, div) in [
            (self.metrics.get_temperature_l3(), L3_TEMP_LABEL, 100),
            (self.metrics.get_current_l3clk(), L3_CLOCK_LABEL, 1),
        ] {
            let Some(val) = val else { continue };
            write!(self.text.buf, " {label:<20}: [")?;
            for v in &val {
                let v = v.saturating_div(div);
                write!(self.text.buf, "{v:5},")?;
            }
            writeln!(self.text.buf, "]")?;
        }

        Ok(())
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.gpu_metrics ^= true;
        }
    }
}
