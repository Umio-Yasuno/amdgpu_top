use super::{DeviceHandle, Text, Opt, PANEL_WIDTH};
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::SENSOR_INFO::*,
};
use std::fmt::{self, Write};
use serde_json::{json, Map, Value};

const WIDTH: usize = PANEL_WIDTH / 2;
const SENSORS_LIST: &[(SENSOR_TYPE, &str, u32)] = &[
    (SENSOR_TYPE::GFX_SCLK, "MHz", 1),
    (SENSOR_TYPE::GFX_MCLK, "MHz", 1),
    // (SENSOR_TYPE::GPU_TEMP, "C", 1000),
    // (SENSOR_TYPE::GPU_LOAD, "%", 1),
    // (SENSOR_TYPE::GPU_AVG_POWER, "W", 1),
    (SENSOR_TYPE::VDDNB, "mV", 1),
    (SENSOR_TYPE::VDDGFX, "mV", 1),
];

// #[derive(Default)]
pub struct Sensor {
    cur: PCI::LINK,
    max: PCI::LINK,
    bus_info: PCI::BUS_INFO,
    power_cap_w: u32,
    critical_temp: u32,
    fan_max_rpm: u32,
    pub text: Text,
}

impl Sensor {
    pub fn new(pci_bus: &PCI::BUS_INFO) -> Self {
        let power_cap_w = Self::parse_hwmon(pci_bus, "power1_cap").saturating_div(1_000_000);
        let critical_temp = Self::parse_hwmon(pci_bus, "temp1_crit").saturating_div(1_000);
        let fan_max_rpm = Self::parse_hwmon(pci_bus, "fan1_max");

        Self {
            cur: pci_bus.get_link_info(PCI::STATUS::Current),
            max: pci_bus.get_link_info(PCI::STATUS::Max),
            bus_info: *pci_bus,
            power_cap_w,
            critical_temp,
            fan_max_rpm,
            text: Text::default(),
        }
    }

    fn parse_hwmon(pci_bus: &PCI::BUS_INFO, file_name: &str) -> u32 {
        pci_bus.get_hwmon_path()
            .and_then(|hwmon_path| std::fs::read_to_string(hwmon_path.join(file_name)).ok())
            .and_then(|file| file.trim_end().parse::<u32>().ok()).unwrap_or(0)
    }

    pub fn update_status(&mut self) {
        self.cur = self.bus_info.get_link_info(PCI::STATUS::Current);
    }

    pub fn print(&mut self, amdgpu_dev: &DeviceHandle) -> Result<(), fmt::Error> {
        const NAME_LEN: usize = 10;
        const VAL_LEN: usize = 5;
        self.text.clear();
        self.update_status();

        let mut c = 0;

        for (sensor, unit, div) in SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                c += 1;
                let val = val.saturating_div(*div);
                write!(
                    self.text.buf,
                    " {:<WIDTH$} ",
                    format!("{sensor_name:<NAME_LEN$} => {val:>VAL_LEN$} {unit:3}")
                )?;
                if (c % 2) == 0 { writeln!(self.text.buf)? };
            }
        }
        if (c % 2) == 1 { writeln!(self.text.buf)?; }

        if let Ok(temp) = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP) {
            writeln!(
                self.text.buf,
                " {name:<NAME_LEN$} => {temp:>VAL_LEN$} C (Crit. {crit} C)",
                name = "GPU Temp",
                temp = temp.saturating_div(1_000),
                crit = self.critical_temp,
            )?;
        }

        if let Ok(power) = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER) {
            writeln!(
                self.text.buf,
                " {name:<NAME_LEN$} => {power:>VAL_LEN$} W (Cap. {cap} W)",
                name = "GPU Power",
                power = power,
                cap = self.power_cap_w,
            )?;
        }

        if let Some(fan_rpm) = self.get_fan_rpm() {
            writeln!(
                self.text.buf,
                " {name:<NAME_LEN$} => {fan_rpm:>VAL_LEN$} RPM (Max. {max} RPM)",
                name = "Fan",
                fan_rpm = fan_rpm,
                max = self.fan_max_rpm,
            )?;
        }

        writeln!(
            self.text.buf,
            " PCI ({pci_bus}) => Gen{cur_gen}x{cur_width:<2} (Max. Gen{max_gen}x{max_width})",
            pci_bus = self.bus_info,
            cur_gen = self.cur.gen,
            cur_width = self.cur.width,
            max_gen = self.max.gen,
            max_width = self.max.width,
        )?;

        Ok(())
    }

    pub fn get_fan_rpm(&self) -> Option<u32> {
        let fan_path = self.bus_info.get_hwmon_path()?.join("fan1_input");

        if let Ok(rpm) = std::fs::read_to_string(fan_path) {
            rpm.trim_end().parse().ok()
        } else {
            None
        }
    }

    pub fn json_value(&self, amdgpu_dev: &DeviceHandle) -> Value {
        let mut m = Map::new();

        m.insert(
            "PCIe Link Speed".to_string(),
            json!({
                "gen": self.cur.gen,
                "width": self.cur.width,
            }),
        );

        for (sensor, unit, div) in SENSORS_LIST {
            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                let val = val.saturating_div(*div);
                m.insert(
                    sensor.to_string(),
                    json!({
                        "value": val,
                        "unit": unit,
                    }),
                );
            }
        }

        if let Some(fan_rpm) = self.get_fan_rpm() {
            m.insert(
                "Fan".to_string(),
                json!({
                    "value": fan_rpm,
                    "unit": "RPM",
                }),
            );
        }

        m.into()
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.sensor ^= true;
        }
    }
}
