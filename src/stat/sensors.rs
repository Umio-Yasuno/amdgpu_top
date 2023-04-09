use super::{DeviceHandle, Text, Opt};
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::SENSOR_INFO::*,
};
use std::fmt::{self, Write};
use serde_json::{json, Map, Value};

const SENSORS_LIST: [(SENSOR_TYPE, &str, u32); 7] = [
    (SENSOR_TYPE::GFX_SCLK, "MHz", 1),
    (SENSOR_TYPE::GFX_MCLK, "MHz", 1),
    (SENSOR_TYPE::GPU_TEMP, "C", 1000),
    (SENSOR_TYPE::GPU_LOAD, "%", 1),
    (SENSOR_TYPE::GPU_AVG_POWER, "W", 1),
    (SENSOR_TYPE::VDDNB, "mV", 1),
    (SENSOR_TYPE::VDDGFX, "mV", 1),
];

// #[derive(Default)]
pub struct Sensor {
    cur: PCI::LINK,
    max: PCI::LINK,
    bus_info: PCI::BUS_INFO,
    pub text: Text,
}

impl Sensor {
    pub fn new(pci_bus: &PCI::BUS_INFO) -> Self {
        Self {
            cur: pci_bus.get_link_info(PCI::STATUS::Current),
            max: pci_bus.get_link_info(PCI::STATUS::Max),
            bus_info: pci_bus.clone(),
            text: Text::default(),
        }
    }

    pub fn update_status(&mut self) {
        self.cur = self.bus_info.get_link_info(PCI::STATUS::Current);
    }

    pub fn print(&mut self, amdgpu_dev: &DeviceHandle) -> Result<(), fmt::Error> {
        self.text.clear();
        self.update_status();

        let mut c = 0;

        for (sensor, unit, div) in &SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                c += 1;
                let val = val.saturating_div(*div);
                let ln = if (c % 2) == 0 { "\n" } else { "" };
                write!(self.text.buf, " {sensor_name:<15} => {val:>6} {unit:3} {ln}")?;
            }
        }
        if (c % 2) == 1 { writeln!(self.text.buf)?; }

        if let Some(power_cap) = self.get_power_cap() {
            let power_cap = power_cap.saturating_div(1_000_000); // microWatts -> Watts
            writeln!(self.text.buf, " {:<15} => {power_cap:>6} W", "PowerCap")?;
        }

        if let Some(fan_rpm) = self.get_fan_rpm() {
            writeln!(self.text.buf, " {:<15} => {fan_rpm:>6} RPM", "FAN1")?;
        }

        writeln!(
            self.text.buf,
            " PCI ({pci_bus}) => Gen{cur_gen}x{cur_width:<2} @ Gen{max_gen}x{max_width:<2} (Max) ",
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

        if let Ok(rpm) = std::fs::read_to_string(&fan_path) {
            rpm.trim_end().parse().ok()
        } else {
            None
        }
    }

    pub fn get_power_cap(&self) -> Option<u32> {
        let power_cap_path = self.bus_info.get_hwmon_path()?.join("power1_cap");

        if let Ok(power_cap) = std::fs::read_to_string(&power_cap_path) {
            power_cap.trim_end().parse().ok()
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

        for (sensor, unit, div) in &SENSORS_LIST {
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
