use libamdgpu_top::AMDGPU::{DeviceHandle};
use super::{PANEL_WIDTH, Text};
use std::fmt::{self, Write};
use crate::Opt;

use libamdgpu_top::stat::{Sensors, PcieBw};

const WIDTH: usize = PANEL_WIDTH / 2;

#[derive(Clone)]
pub struct SensorsView {
    sensors: Sensors,
    pub text: Text,
}

impl SensorsView {
/*
    pub fn _new(amdgpu_dev: &DeviceHandle, pci_bus: &PCI::BUS_INFO) -> Self {
        Self {
            sensors: Sensors::new(amdgpu_dev, pci_bus),
            text: Text::default(),
        }
    }
*/
    pub fn new_with_sensors(sensors: Sensors) -> Self {
        Self {
            sensors,
            text: Text::default(),
        }
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.sensors.update(amdgpu_dev);
    }

    pub fn print(&mut self) -> Result<(), fmt::Error> {
        let sensors = &self.sensors;
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
                " {:<WIDTH$} ",
                format!("{name:<NAME_LEN$} => {val:>VAL_LEN$} {unit:3}")
            )?;
            if (c % 2) == 0 { writeln!(self.text.buf)? };
        }
        if (c % 2) == 1 { writeln!(self.text.buf)?; }

        if let Some(power) = sensors.power {
            write!(self.text.buf, " GPU Power  => {power:3} W")?;
            if let Some(ref cap) = sensors.power_cap {
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
                write!(self.text.buf, " (Max. {max_rpm} RPM)")?;
            }
            writeln!(self.text.buf)?;
        }

        if let [Some(cur), Some(max)] = [sensors.cur, sensors.max] {
            writeln!(
                self.text.buf,
                " PCIe Link Speed => Gen{cur_gen}x{cur_width:<2} (Max. Gen{max_gen}x{max_width})",
                cur_gen = cur.gen,
                cur_width = cur.width,
                max_gen = max.gen,
                max_width = max.width,
            )?;
        }

        Ok(())
    }

    pub fn print_pcie_bw(&mut self, pcie_bw: &PcieBw) -> Result<(), fmt::Error> {
        let sent = (pcie_bw.sent * pcie_bw.max_payload_size as u64) >> 20; // MiB
        let rec = (pcie_bw.received * pcie_bw.max_payload_size as u64) >> 20; // MiB

        writeln!(
            self.text.buf,
            " PCIe Bandwidth Usage => Sent: {sent:6} MiB/s, Received: {rec:6} MiB/s",
        )?;

        Ok(())
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.sensor ^= true;
        }
    }
}
