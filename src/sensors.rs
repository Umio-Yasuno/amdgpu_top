use crate::AMDGPU::{
    DeviceHandle,
    SENSOR_INFO::*,
};

const NA: &str = "n/a";
// TODO: check asic name?
const SENSORS_LIST: [(SENSOR_TYPE, &str, u32); 7] = [
    (SENSOR_TYPE::GFX_SCLK, "MHz", 1),
    (SENSOR_TYPE::GFX_MCLK, "MHz", 1),
    (SENSOR_TYPE::GPU_TEMP, "C", 1000),
    (SENSOR_TYPE::GPU_LOAD, "%", 1),
    (SENSOR_TYPE::GPU_AVG_POWER, "W", 1),
    (SENSOR_TYPE::VDDNB, "mV", 1),
    (SENSOR_TYPE::VDDGFX, "mV", 1),
];

pub struct Sensor;

impl Sensor {
    pub fn stat(amdgpu_dev: &DeviceHandle) -> String {
        let mut s = String::from("\n");

        for (sensor, unit, div) in &SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                let val = val.saturating_div(*div);
                s.push_str(&format!(" {sensor_name:<15} {val:>6} {unit:3} \n"));
            } else {
                s.push_str(&format!(" {sensor_name:<15} {NA:>10} \n"));
            }
        }

        s
    }
}
