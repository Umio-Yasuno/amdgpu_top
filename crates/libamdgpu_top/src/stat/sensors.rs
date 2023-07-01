use std::path::PathBuf;
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::{
        ASIC_NAME,
        DeviceHandle,
        HwmonTemp,
        HwmonTempType,
        SENSOR_INFO::SENSOR_TYPE,
        PowerCap,
    },
};
use super::parse_hwmon;

#[derive(Clone, Debug)]
pub struct Sensors {
    pub hwmon_path: PathBuf,
    pub vega10_and_later: bool,
    pub cur: Option<PCI::LINK>,
    pub max: Option<PCI::LINK>,
    pub bus_info: PCI::BUS_INFO,
    pub sclk: Option<u32>,
    pub mclk: Option<u32>,
    pub vddnb: Option<u32>,
    pub vddgfx: Option<u32>,
    pub edge_temp: Option<HwmonTemp>,
    pub junction_temp: Option<HwmonTemp>,
    pub memory_temp: Option<HwmonTemp>,
    pub power: Option<u32>,
    pub power_cap: Option<PowerCap>,
    pub fan_rpm: Option<u32>,
    pub fan_max_rpm: Option<u32>,
}

impl Sensors {
    pub fn new(amdgpu_dev: &DeviceHandle, pci_bus: &PCI::BUS_INFO, asic_name: ASIC_NAME) -> Self {
        let hwmon_path = pci_bus.get_hwmon_path().unwrap();
        let vega10_and_later = ASIC_NAME::CHIP_VEGA10 <= asic_name;

        // AMDGPU driver reports maximum number of PCIe lanes of Polaris11/Polaris12 as x16
        // in `pp_dpm_pcie` (actually x8), so we use `{current,max}_link_{speed,width}`.  
        // ref: drivers/gpu/drm/amd/pm/powerplay/hwmgr/smu7_hwmgr.c
        // 
        // However, recent AMD GPUs have multiple endpoints, and the correct PCIe speed/width
        // for the GPU is output to `pp_dpm_pcie`.  
        // ref: <https://gitlab.freedesktop.org/drm/amd/-/issues/1967>
        let [cur, max] = if vega10_and_later {
            let max = pci_bus.get_min_max_link_info_from_dpm().map(|[_min, max]| max);

            [
                pci_bus.get_current_link_info_from_dpm(),
                max,
            ]
        } else {
            [
                Some(pci_bus.get_link_info(PCI::STATUS::Current)),
                Some(pci_bus.get_link_info(PCI::STATUS::Max)),
            ]
        };

        let [sclk, mclk, vddnb, vddgfx, power] = [
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok(),
        ];
        let edge_temp = HwmonTemp::from_hwmon_path(&hwmon_path, HwmonTempType::Edge);
        let junction_temp = HwmonTemp::from_hwmon_path(&hwmon_path, HwmonTempType::Junction);
        let memory_temp = HwmonTemp::from_hwmon_path(&hwmon_path, HwmonTempType::Memory);
        let power_cap = PowerCap::from_hwmon_path(&hwmon_path);

        let fan_rpm = parse_hwmon(hwmon_path.join("fan1_input"));
        let fan_max_rpm = parse_hwmon(hwmon_path.join("fan1_max"));

        Self {
            hwmon_path,
            vega10_and_later,
            cur,
            max,
            bus_info: *pci_bus,
            sclk,
            mclk,
            vddnb,
            vddgfx,
            edge_temp,
            junction_temp,
            memory_temp,
            power,
            power_cap,
            fan_rpm,
            fan_max_rpm,
        }
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.cur = if self.vega10_and_later {
            self.bus_info.get_current_link_info_from_dpm()
        } else {
            Some(self.bus_info.get_link_info(PCI::STATUS::Current))
        };
        self.sclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok();
        self.mclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok();
        self.vddnb = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok();
        self.vddgfx = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok();

        for temp in [&mut self.edge_temp, &mut self.junction_temp, &mut self.memory_temp] {
            let Some(temp) = temp else { continue };
            temp.update(&self.hwmon_path);
        }

        self.power = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok();
        self.fan_rpm = parse_hwmon(self.hwmon_path.join("fan1_input"));
    }
}
