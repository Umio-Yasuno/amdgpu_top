use crate::util::Text;
use crate::AMDGPU::{
    DeviceHandle,
    SENSOR_INFO::*,
};

const NA: &str = "n/a";
const SENSORS_LIST: [(SENSOR_TYPE, &str, u32); 7] = [
    (SENSOR_TYPE::GFX_SCLK, "MHz", 1),
    (SENSOR_TYPE::GFX_MCLK, "MHz", 1),
    (SENSOR_TYPE::GPU_TEMP, "C", 1000),
    (SENSOR_TYPE::GPU_LOAD, "%", 1),
    (SENSOR_TYPE::GPU_AVG_POWER, "W", 1),
    (SENSOR_TYPE::VDDNB, "mV", 1),
    (SENSOR_TYPE::VDDGFX, "mV", 1),
];

#[derive(Default)]
pub struct Sensor {
    pub text: Text,
}

impl Sensor {
    pub fn print(&mut self, amdgpu_dev: &DeviceHandle) {
        use std::fmt::Write;

        self.text.clear();

        for (sensor, unit, div) in &SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                let val = val.saturating_div(*div);
                writeln!(self.text.buf, " {sensor_name:<15} => {val:>6} {unit:3} ").unwrap();
            } else {
                writeln!(self.text.buf, " {sensor_name:<15} => {NA:^10} ").unwrap();
            }
        }
    }
}
