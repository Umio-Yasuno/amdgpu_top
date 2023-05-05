use std::path::PathBuf;
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::{DeviceHandle, SENSOR_INFO::SENSOR_TYPE},
};

#[derive(Clone, Debug)]
pub struct Sensors {
    pub hwmon_path: PathBuf,
    pub cur: PCI::LINK,
    pub max: PCI::LINK,
    pub bus_info: PCI::BUS_INFO,
    pub sclk: Option<u32>,
    pub mclk: Option<u32>,
    pub vddnb: Option<u32>,
    pub vddgfx: Option<u32>,
    pub temp: Option<u32>,
    pub critical_temp: Option<u32>,
    pub power: Option<u32>,
    pub power_cap: Option<u32>,
    pub fan_rpm: Option<u32>,
    pub fan_max_rpm: Option<u32>,
}

impl Sensors {
    pub fn new(amdgpu_dev: &DeviceHandle, pci_bus: &PCI::BUS_INFO) -> Self {
        let hwmon_path = pci_bus.get_hwmon_path().unwrap();
        let cur = pci_bus.get_link_info(PCI::STATUS::Current);
        let max = pci_bus.get_link_info(PCI::STATUS::Max);
        let [sclk, mclk, vddnb, vddgfx, temp, power] = [
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok(),
        ];
        let critical_temp = Self::parse_hwmon(hwmon_path.join("temp1_crit"))
            .map(|temp| temp.saturating_div(1_000));
        let power_cap = Self::parse_hwmon(hwmon_path.join("power1_cap"))
            .map(|cap| cap.saturating_div(1_000_000));

        let fan_rpm = Self::parse_hwmon(hwmon_path.join("fan1_input"));
        let fan_max_rpm = Self::parse_hwmon(hwmon_path.join("fan1_max"));

        Self {
            hwmon_path,
            cur,
            max,
            bus_info: *pci_bus,
            sclk,
            mclk,
            vddnb,
            vddgfx,
            temp,
            critical_temp,
            power,
            power_cap,
            fan_rpm,
            fan_max_rpm,
        }
    }

    fn parse_hwmon<P: Into<PathBuf>>(path: P) -> Option<u32> {
        std::fs::read_to_string(path.into()).ok()
            .and_then(|file| file.trim_end().parse::<u32>().ok())
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.cur = self.bus_info.get_link_info(PCI::STATUS::Current);
        self.sclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok();
        self.mclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok();
        self.vddnb = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok();
        self.vddgfx = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok();
        self.temp = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP).ok();
        self.power = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok();
        self.fan_rpm = Self::parse_hwmon(self.hwmon_path.join("fan1_input"));
    }
}
