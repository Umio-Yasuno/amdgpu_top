use super::{DeviceHandle, Text, Opt};
use libdrm_amdgpu_sys::AMDGPU::{
    SENSOR_INFO::*,
};
use libdrm_amdgpu_sys::PCI;
use std::path::PathBuf;

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

    pub fn print(&mut self, amdgpu_dev: &DeviceHandle) {
        use std::fmt::Write;

        self.text.clear();
        self.update_status();

        writeln!(
            self.text.buf,
            " PCI ({pci_bus}) => Gen{cur_gen}x{cur_width:<2} @ Gen{max_gen}x{max_width:<2} (Max) ",
            pci_bus = self.bus_info,
            cur_gen = self.cur.gen,
            cur_width = self.cur.width,
            max_gen = self.max.gen,
            max_width = self.max.width,
        ).unwrap();

        let mut c = 0;

        for (sensor, unit, div) in &SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                c += 1;
                let val = val.saturating_div(*div);
                let ln = if (c % 2) == 0 { "\n" } else { "" };
                write!(self.text.buf, " {sensor_name:<15} => {val:>6} {unit:3} {ln}").unwrap();
            }
        }

        if let Some(fan_rpm) = self.get_fan_rpm() {
            writeln!(self.text.buf, " {:<15} => {fan_rpm:>6} RPM", "FAN1").unwrap();
        }
    }

    pub fn get_fan_rpm(&self) -> Option<u32> {
        let path = format!("/sys/bus/pci/devices/{}/hwmon/", self.bus_info);
        let Ok(mut dir) = std::fs::read_dir(&path) else { return None };
        let Some(hwmon_dir) = dir.next() else { return None };
        let Ok(hwmon_dir) = hwmon_dir else { return None };

        let fan_path: PathBuf = [hwmon_dir.path(), PathBuf::from("fan1_input")].iter().collect();

        if let Ok(rpm) = std::fs::read_to_string(&fan_path) {
            rpm.trim_end().parse().ok()
        } else {
            None
        }
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.sensor ^= true;
        }
    }
}
