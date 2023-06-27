use std::fmt::{self, Write};
use super::Text;
use crate::Opt;
use libamdgpu_top::AMDGPU::{DeviceHandle, GpuMetrics, MetricsInfo};
use libamdgpu_top::stat::gpu_metrics_util::*;
use std::path::PathBuf;

const CORE_TEMP_LABEL: &str = "Core Temp (C)";
const CORE_POWER_LABEL: &str = "Core Power (mW)";
const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

#[derive(Clone)]
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

    pub fn version(&self) -> Option<(u8, u8)> {
        let header = self.metrics.get_header()?;

        Some((header.format_revision, header.content_revision))
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

        if let Some(thr) = self.metrics.get_throttle_status_info() {
            writeln!(
                self.text.buf,
                " Throttle Status: {:?}",
                thr.get_all_throttler(),
            )?;
        }

        Ok(())
    }

    // AMDGPU always returns `u16::MAX` for some values it doesn't actually support.
    fn for_v1(&mut self) -> Result<(), fmt::Error> {
        socket_power(&mut self.text.buf, &self.metrics)?;
        avg_activity(&mut self.text.buf, &self.metrics)?;

        v1_helper(&mut self.text.buf, "C", &[
            (self.metrics.get_temperature_vrgfx(), "VRGFX"),
            (self.metrics.get_temperature_vrsoc(), "VRSOC"),
            (self.metrics.get_temperature_vrmem(), "VRMEM"),
        ])?;

        v1_helper(&mut self.text.buf, "mV", &[
            (self.metrics.get_voltage_soc(), "SoC"),
            (self.metrics.get_voltage_gfx(), "GFX"),
            (self.metrics.get_voltage_mem(), "Mem"),
        ])?;

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
                "UMCCLK",
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
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            writeln!(self.text.buf, " {name:<6} => Avg. {avg:4} MHz, Cur. {cur:4} MHz")?;
        }

        // Only Aldebaran (MI200) supports it.
        if let Some(hbm_temp) = check_hbm_temp(self.metrics.get_temperature_hbm()) {
            write!(self.text.buf, "HBM Temp (C) => [")?;
            for v in &hbm_temp {
                write!(self.text.buf, "{v:5},")?;
            }
            writeln!(self.text.buf, "]")?;
        }

        Ok(())
    }

    fn for_v2(&mut self) -> Result<(), fmt::Error> {
        let temp_gfx = self.metrics.get_temperature_gfx().map(|v| v.saturating_div(100));
        let temp_soc = self.metrics.get_temperature_soc().map(|v| v.saturating_div(100));

        write!(self.text.buf, " GFX => ")?;
        v2_helper(&mut self.text.buf, &[
            (temp_gfx, "C"),
            (self.metrics.get_average_gfx_power(), "mW"),
            (self.metrics.get_current_gfxclk(), "MHz"),
        ])?;

        write!(self.text.buf, " SoC => ")?;
        v2_helper(&mut self.text.buf, &[
            (temp_soc, "C"),
            (self.metrics.get_average_soc_power(), "mW"),
            (self.metrics.get_current_socclk(), "MHz"),
        ])?;

        /*
            Most APUs return `average_socket_power` in mW,
            but Renoir APU (Renoir, Lucienne, Cezanne, Barcelo) return in W
            depending on the power management firmware version.  

            ref: drivers/gpu/drm/amd/pm/swsmu/smu12/renoir_ppt.c
            ref: https://gitlab.freedesktop.org/drm/amd/-/issues/2321
        */
        // socket_power(&mut self.text.buf, &self.metrics)?;
        avg_activity(&mut self.text.buf, &self.metrics)?;

        for (avg, cur, name) in [
            (
                self.metrics.get_average_uclk_frequency(),
                self.metrics.get_current_uclk(),
                "UMCCLK",
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
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            writeln!(self.text.buf, " {name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz")?;
        }

        let core_temp = check_temp_array(self.metrics.get_temperature_core());
        let l3_temp = check_temp_array(self.metrics.get_temperature_l3());
        let [core_power, core_clk] = [
            self.metrics.get_average_core_power(),
            self.metrics.get_current_coreclk(),
        ].map(check_power_clock_array);
        let l3_clk = check_power_clock_array(self.metrics.get_current_l3clk());

        for (val, label) in [
            (core_temp, CORE_TEMP_LABEL),
            (core_power, CORE_POWER_LABEL),
            (core_clk, CORE_CLOCK_LABEL),
        ] {
            let Some(val) = val else { continue };
            write!(self.text.buf, " {label:<16} => [")?;
            for v in &val {
                write!(self.text.buf, "{v:5},")?;
            }
            writeln!(self.text.buf, "]")?;
        }

        for (val, label) in [
            (l3_temp, L3_TEMP_LABEL),
            (l3_clk, L3_CLOCK_LABEL),
        ] {
            let Some(val) = val else { continue };
            write!(self.text.buf, " {label:<20} => [")?;
            for v in &val {
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

fn socket_power(buf: &mut String, gpu_metrics: &GpuMetrics) -> Result<(), fmt::Error> {
    let v = check_metrics_val(gpu_metrics.get_average_socket_power());
    writeln!(buf, " Socket Power => {v:>3} W")
}

fn avg_activity(buf: &mut String, gpu_metrics: &GpuMetrics) -> Result<(), fmt::Error> {
    write!(buf, " Average Activity => ")?;
    for (val, label) in [
        (gpu_metrics.get_average_gfx_activity(), "GFX"),
        (gpu_metrics.get_average_umc_activity(), "UMC"),
        (gpu_metrics.get_average_mm_activity(), "Media"),
    ] {
        let v = check_metrics_val(val.map(|v| v.saturating_div(100)));
        write!(buf, "{label} {v:>3}%, ")?;
    }
    writeln!(buf)
}

fn v1_helper(buf: &mut String, unit: &str, v: &[(Option<u16>, &str)]) -> Result<(), fmt::Error> {
    for (val, name) in v {
        let v = check_metrics_val(*val);
        write!(buf, " {name} => {v:>4} {unit}, ")?;
    }
    writeln!(buf)
}

fn v2_helper(buf: &mut String, v: &[(Option<u16>, &str)]) -> Result<(), fmt::Error> {
    for (val, unit) in v {
        let v = check_metrics_val(*val);
        write!(buf, "{v:>5} {unit}, ")?;
    }
    writeln!(buf)
}
