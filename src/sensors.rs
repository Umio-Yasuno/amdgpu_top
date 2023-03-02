use crate::AMDGPU::{
    DeviceHandle,
    SENSOR_INFO::*,
};

const SENSORS_LIST: [SENSOR_TYPE; 7] = [
    SENSOR_TYPE::GFX_SCLK,
    SENSOR_TYPE::GFX_MCLK,
    SENSOR_TYPE::GPU_TEMP,
    SENSOR_TYPE::GPU_LOAD,
    SENSOR_TYPE::GPU_AVG_POWER,
    SENSOR_TYPE::VDDNB,
    SENSOR_TYPE::VDDGFX,
];

pub struct Sensor;

impl Sensor {
    pub fn stat(amdgpu_dev: &DeviceHandle) -> String {
        let mut s = String::new();

        for sensor in &SENSORS_LIST {
            let sensor_name = sensor.to_string();

            if let Ok(val) = amdgpu_dev.sensor_info(*sensor) {
                s.push_str(&format!(" {sensor_name:<15} {val:>6} \n"));
            } else {
                s.push_str(&format!(" {sensor_name:<15} {:>6} \n", "n/a"));
            }
        }

        s
    }
}
