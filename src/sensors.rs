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

pub struct Sensor {
    pub(crate) buf: String,
}

impl Default for Sensor {
    fn default() -> Self {
        Self {
            buf: String::new(),
        }
    }
}

impl Sensor {
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn stat(&mut self, amdgpu_dev: &DeviceHandle) {
        use std::fmt::Write;

        self.buf.clear();

        for (sensor, unit, div) in &SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                let val = val.saturating_div(*div);
                writeln!(self.buf, " {sensor_name:<15} => {val:>6} {unit:3} ").unwrap();
            } else {
                writeln!(self.buf, " {sensor_name:<15} => {NA:^10} ").unwrap();
            }
        }
    }
}
