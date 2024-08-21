use super::PANEL_WIDTH;
use std::fmt::{self, Write};
use crate::Opt;

use libamdgpu_top::stat::{Sensors, PcieBw};

const WIDTH: usize = PANEL_WIDTH / 2;

use crate::AppTextView;

impl AppTextView {
    pub fn print_sensors(&mut self, sensors: &Sensors) -> Result<(), fmt::Error> {
        const NAME_LEN: usize = 10;
        const VAL_LEN: usize = 5;
        self.text.clear();

        let mut c = 0;

        for (name, val, unit) in [
            ("GFX_SCLK", sensors.sclk, "MHz"),
            ("GFX_MCLK", sensors.mclk, "MHz"),
            ("VDDNB", sensors.vddnb, "mV"),
            ("VDDGFX", sensors.vddgfx, "mV"),
        ] {
            let Some(val) = val else { continue };
            c += 1;
            write!(
                self.text.buf,
                " {:<WIDTH$}",
                format!("{name:<NAME_LEN$} => {val:>VAL_LEN$} {unit:3}")
            )?;
            if (c % 2) == 0 { writeln!(self.text.buf)? };
        }
        if (c % 2) == 1 { writeln!(self.text.buf)?; }

        if sensors.average_power.is_some() || sensors.input_power.is_some() {
            write!(self.text.buf, " GPU Power  =>")?;

            for power in [&sensors.average_power, &sensors.input_power] {
                let Some(power) = power else { continue };
                write!(
                    self.text.buf,
                    " {:3} W ({})",
                    power.value,
                    power.type_,
                )?;
            }

            if let Some(cap) = &sensors.power_cap {
                write!(
                    self.text.buf,
                    " (Cap. {} W, {}-{} W)", cap.current, cap.min, cap.max,
                )?;
            }
            writeln!(self.text.buf)?;
        }

        for temp in [&sensors.edge_temp, &sensors.junction_temp, &sensors.memory_temp] {
            let Some(temp) = temp else { continue };
            let label = format!("{} Temp.", temp.type_);
            write!(self.text.buf, " {label:<15} => {:3} C", temp.current)?;
            if let Some(crit) = temp.critical {
                write!(self.text.buf, " (Crit. {crit} C)")?;
            }
            if let Some(e) = temp.emergency {
                write!(self.text.buf, " (Emergency {e} C)")?;
            }
            writeln!(self.text.buf)?;
        }

        if let Some(fan_rpm) = sensors.fan_rpm {
            write!(self.text.buf, " Fan => {fan_rpm:4} RPM")?;
            if let Some(max_rpm) = sensors.fan_max_rpm {
                if let Some(per) = fan_rpm.saturating_mul(100).checked_div(max_rpm) {
                    write!(self.text.buf, " ({per:>3}%)")?;
                }

                write!(self.text.buf, " (Max. {max_rpm} RPM)")?;
            }
            writeln!(self.text.buf)?;
        }

        if let Some(cur) = sensors.current_link {
            write!(self.text.buf, " PCIe Link Speed => Gen{}x{:<2}", cur.gen, cur.width)?;

            if let [Some(min), Some(max)] = [sensors.min_dpm_link, sensors.max_dpm_link] {
                write!(
                    self.text.buf,
                    " (Gen{}x{} - Gen{}x{})",
                    min.gen,
                    min.width,
                    max.gen,
                    max.width,
                )?;
            } else if let Some(max) = sensors.max_dpm_link {
                write!(self.text.buf, " (Max. Gen{}x{})", max.gen, max.width)?;
            }

            writeln!(self.text.buf)?;
        }

        if let Some(power_state) = &sensors.pci_power_state {
            writeln!(self.text.buf, " PCI Power State: {power_state}")?;
        }

        if let Some(power_profile) = &sensors.power_profile {
            writeln!(self.text.buf, " Power Profile: {power_profile}")?;
        }

        Ok(())
    }

    pub fn print_pcie_bw(&mut self, pcie_bw: &PcieBw) -> Result<(), fmt::Error> {
        let Some(mps) = pcie_bw.max_payload_size else { return Ok(()) };
        let Some(sent) = pcie_bw.sent.map(|v| (v * mps as u64) >> 20) else { return Ok(()) };
        let Some(rec) = pcie_bw.received.map(|v| (v * mps as u64) >> 20) else { return Ok(()) };

        writeln!(
            self.text.buf,
            " PCIe Bandwidth Usage => Sent: {sent:6} MiB/s, Received: {rec:6} MiB/s",
        )?;

        Ok(())
    }

    pub fn cb_sensors(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.sensor ^= true;
        }
    }
}
