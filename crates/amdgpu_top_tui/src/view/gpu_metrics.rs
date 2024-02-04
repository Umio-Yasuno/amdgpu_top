use std::fmt::{self, Write};
use crate::Opt;
use libamdgpu_top::AMDGPU::{GpuMetrics, MetricsInfo};
use libamdgpu_top::stat::{gpu_metrics_util::*, GpuActivity};

use crate::AppTextView;

const CORE_TEMP_LABEL: &str = "Core Temp (C)";
const CORE_POWER_LABEL: &str = "Core Power (mW)";
const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

impl AppTextView {
    pub fn print_gpu_metrics(&mut self, metrics: &GpuMetrics) -> Result<(), fmt::Error> {
        self.text.clear();

        match metrics {
            GpuMetrics::V1_0(_) |
            GpuMetrics::V1_1(_) |
            GpuMetrics::V1_2(_) |
            GpuMetrics::V1_3(_) |
            GpuMetrics::V1_4(_) |
            GpuMetrics::V1_5(_) => self.gpu_metrics_v1_x(metrics)?,
            GpuMetrics::V2_0(_) |
            GpuMetrics::V2_1(_) |
            GpuMetrics::V2_2(_) |
            GpuMetrics::V2_3(_) |
            GpuMetrics::V2_4(_) => self.gpu_metrics_v2_x(metrics)?,
            _ => {},
        };

        if let Some(thr) = metrics.get_throttle_status_info() {
            writeln!(
                self.text.buf,
                " Throttle Status: {:?}",
                thr.get_all_throttler(),
            )?;
        }

        Ok(())
    }

    // AMDGPU always returns `u16::MAX` for some values it doesn't actually support.
    fn gpu_metrics_v1_x(&mut self, metrics: &GpuMetrics) -> Result<(), fmt::Error> {
        socket_power(&mut self.text.buf, metrics)?;
        avg_activity(&mut self.text.buf, metrics)?;

        v1_helper(&mut self.text.buf, "C", &[
            (metrics.get_temperature_vrgfx(), "VRGFX"),
            (metrics.get_temperature_vrsoc(), "VRSOC"),
            (metrics.get_temperature_vrmem(), "VRMEM"),
        ])?;

        v1_helper(&mut self.text.buf, "mV", &[
            (metrics.get_voltage_soc(), "SoC"),
            (metrics.get_voltage_gfx(), "GFX"),
            (metrics.get_voltage_mem(), "Mem"),
        ])?;

        for (avg, cur, name) in [
            (
                metrics.get_average_gfxclk_frequency(),
                metrics.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                metrics.get_average_socclk_frequency(),
                metrics.get_current_socclk(),
                "SOCCLK",
            ),
            (
                metrics.get_average_uclk_frequency(),
                metrics.get_current_uclk(),
                "UMCCLK",
            ),
            (
                metrics.get_average_vclk_frequency(),
                metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                metrics.get_average_dclk_frequency(),
                metrics.get_current_dclk(),
                "DCLK",
            ),
            (
                metrics.get_average_vclk1_frequency(),
                metrics.get_current_vclk1(),
                "VCLK1",
            ),
            (
                metrics.get_average_dclk1_frequency(),
                metrics.get_current_dclk1(),
                "DCLK1",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            writeln!(self.text.buf, " {name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz")?;
        }

        // Only Aldebaran (MI200) supports it.
        if let Some(hbm_temp) = check_hbm_temp(metrics.get_temperature_hbm()) {
            write!(self.text.buf, "HBM Temp (C) => [")?;
            for v in &hbm_temp {
                write!(self.text.buf, "{v:5},")?;
            }
            writeln!(self.text.buf, "]")?;
        }

        match metrics {
            GpuMetrics::V1_4(_) |
            GpuMetrics::V1_5(_) => self.gpu_metrics_v1_4_v1_5(metrics)?,
            _ => {},
        }

        Ok(())
    }

    fn gpu_metrics_v1_4_v1_5(&mut self, metrics: &GpuMetrics) -> Result<(), fmt::Error> {
        if let Some(all_gfxclk) = metrics.get_all_instances_current_gfxclk() {
            writeln!(self.text.buf, "GFXCLK (Current) => [{}]", all_clk_helper(&all_gfxclk))?;
        }

        for (label, all_clk) in [
            ("SOCCLK (Current)", metrics.get_all_instances_current_socclk()),
            ("VCLK0 (Current) ", metrics.get_all_instances_current_vclk0()),
            ("DCLK0 (Current) ", metrics.get_all_instances_current_dclk0()),
        ] {
            let Some(all_clk) = all_clk else { continue };
            writeln!(self.text.buf, "{label} => [{}]", all_clk_helper(&all_clk))?;
        }

        if let Some(all_vcn) = metrics.get_all_vcn_activity() {
            writeln!(self.text.buf, "VCN Activity => [{}]", all_activity_helper(&all_vcn))?;
        }

        if let Some(all_jpeg) = metrics.get_all_jpeg_activity() {
            writeln!(self.text.buf, "JPEG Activity => [{}]", all_activity_helper(&all_jpeg))?;
        }

        if let [Some(xgmi_width), Some(xgmi_speed)] = [
            metrics.get_xgmi_link_width(),
            metrics.get_xgmi_link_speed(),
        ] {
            writeln!(self.text.buf, "XGMI => x{xgmi_width} {xgmi_speed}Gbps")?;
        }

        Ok(())
    }

    fn gpu_metrics_v2_x(&mut self, metrics: &GpuMetrics) -> Result<(), fmt::Error> {
        let temp_gfx = metrics.get_temperature_gfx().map(|v| v.saturating_div(100));
        let temp_soc = metrics.get_temperature_soc().map(|v| v.saturating_div(100));

        write!(self.text.buf, " CPU => {pad:9}", pad = "")?;
        v2_helper(&mut self.text.buf, &[(metrics.get_average_cpu_power(), "mW")])?;

        write!(self.text.buf, " GFX => ")?;
        v2_helper(&mut self.text.buf, &[
            (temp_gfx, "C"),
            (metrics.get_average_gfx_power(), "mW"),
            (metrics.get_current_gfxclk(), "MHz"),
        ])?;

        write!(self.text.buf, " SoC => ")?;
        v2_helper(&mut self.text.buf, &[
            (temp_soc, "C"),
            (metrics.get_average_soc_power(), "mW"),
            (metrics.get_current_socclk(), "MHz"),
        ])?;

        /*
            Most APUs return `average_socket_power` in mW,
            but Renoir APU (Renoir, Lucienne, Cezanne, Barcelo) return in W
            depending on the power management firmware version.  

            ref: drivers/gpu/drm/amd/pm/swsmu/smu12/renoir_ppt.c
            ref: https://gitlab.freedesktop.org/drm/amd/-/issues/2321
        */
        // socket_power(&mut self.text.buf, metrics)?;
        avg_activity(&mut self.text.buf, metrics)?;

        for (avg, cur, name) in [
            (
                metrics.get_average_uclk_frequency(),
                metrics.get_current_uclk(),
                "UMCCLK",
            ),
            (
                metrics.get_average_fclk_frequency(),
                metrics.get_current_fclk(),
                "FCLK",
            ),
            (
                metrics.get_average_vclk_frequency(),
                metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                metrics.get_average_dclk_frequency(),
                metrics.get_current_dclk(),
                "DCLK",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            writeln!(self.text.buf, " {name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz")?;
        }

        let core_temp = check_temp_array(metrics.get_temperature_core());
        let l3_temp = check_temp_array(metrics.get_temperature_l3());
        let [core_power, core_clk] = [
            metrics.get_average_core_power(),
            metrics.get_current_coreclk(),
        ].map(check_power_clock_array);
        let l3_clk = check_power_clock_array(metrics.get_current_l3clk());

        for (val, label) in [
            (core_temp, CORE_TEMP_LABEL),
            (core_power, CORE_POWER_LABEL),
            (core_clk, CORE_CLOCK_LABEL),
        ] {
            let Some(val) = val else { continue };
            let s = val.iter().fold(String::new(), |mut s, v| {
                let _ = write!(s, "{v:>5},");
                s
            });
            writeln!(self.text.buf, " {label:<16} => [{s}]")?;
        }

        for (val, label) in [
            (l3_temp, L3_TEMP_LABEL),
            (l3_clk, L3_CLOCK_LABEL),
        ] {
            let Some(val) = val else { continue };
            let s = val.iter().fold(String::new(), |mut s, v| {
                let _ = write!(s, "{v:>5},");
                s
            });
            writeln!(self.text.buf, " {label:<20} => [{s}]")?;
        }

        for (label, voltage, current) in [
            ("CPU", metrics.get_average_cpu_voltage(), metrics.get_average_cpu_current()),
            ("SoC", metrics.get_average_soc_voltage(), metrics.get_average_soc_current()),
            ("GFX", metrics.get_average_gfx_voltage(), metrics.get_average_gfx_current()),
        ] {
            let Some(voltage) = voltage else { continue };
            let Some(current) = current else { continue };

            writeln!(self.text.buf, " {label} => {voltage:>5} mV, {current:>5} mA")?;
        }

        Ok(())
    }

    pub fn cb_gpu_metrics(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.gpu_metrics ^= true;
        }
    }
}

fn socket_power(buf: &mut String, gpu_metrics: &GpuMetrics) -> Result<(), fmt::Error> {
    let avg = check_metrics_val(gpu_metrics.get_average_socket_power());
    writeln!(buf, " Socket Power (Average) => {avg:>3} W")?;

    match gpu_metrics {
        GpuMetrics::V1_4(_) |
        GpuMetrics::V1_5(_) => {
            let cur = check_metrics_val(gpu_metrics.get_current_socket_power());
            writeln!(buf, " Socket Power (Current) => {cur:>3} W")?;
        },
        _ => {},
    }

    Ok(())
}

fn avg_activity(buf: &mut String, gpu_metrics: &GpuMetrics) -> Result<(), fmt::Error> {
    write!(buf, " Average Activity => ")?;
    let activity = GpuActivity::from_gpu_metrics(gpu_metrics);

    for (val, label) in [
        (activity.gfx, "GFX"),
        (activity.umc, "UMC"),
        (activity.media, "Media"),
    ] {
        if let Some(val) = val {
            write!(buf, "{label} {val:>3}%, ")?;
        } else {
            write!(buf, "{label} ___%, ")?;
        }
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
