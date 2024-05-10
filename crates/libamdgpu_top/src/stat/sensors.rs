use std::path::PathBuf;
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::{
        drm_amdgpu_info_device,
        GPU_INFO,
        ASIC_NAME,
        DeviceHandle,
        HwmonTemp,
        HwmonTempType,
        SENSOR_INFO::SENSOR_TYPE,
        PowerCap,
    },
};
use super::{parse_hwmon, HwmonPower, PowerType};

#[derive(Clone, Debug)]
pub struct Sensors {
    pub hwmon_path: PathBuf,
    pub is_apu: bool,
    pub vega10_and_later: bool,
    pub current_link: Option<PCI::LINK>,
    pub min_dpm_link: Option<PCI::LINK>,
    pub max_dpm_link: Option<PCI::LINK>,
    pub max_gpu_link: Option<PCI::LINK>,
    pub max_system_link: Option<PCI::LINK>,
    pub bus_info: PCI::BUS_INFO,
    pub sclk: Option<u32>,
    pub mclk: Option<u32>,
    pub vddnb: Option<u32>,
    pub vddgfx: Option<u32>,
    pub edge_temp: Option<HwmonTemp>,
    pub junction_temp: Option<HwmonTemp>,
    pub memory_temp: Option<HwmonTemp>,
    pub average_power: Option<HwmonPower>,
    pub input_power: Option<HwmonPower>,
    pub power_cap: Option<PowerCap>,
    pub fan_rpm: Option<u32>,
    pub fan_max_rpm: Option<u32>,
    pub gpu_port_path: PathBuf,
    pub pci_power_state: Option<String>,
}

impl Sensors {
    pub fn new(
        amdgpu_dev: &DeviceHandle,
        pci_bus: &PCI::BUS_INFO,
        ext_info: &drm_amdgpu_info_device,
    ) -> Option<Self> {
        let hwmon_path = pci_bus.get_hwmon_path()?;
        let asic_name = ext_info.get_asic_name();
        let is_apu = ext_info.is_apu();
        let vega10_and_later = ASIC_NAME::CHIP_VEGA10 <= asic_name;

        // The AMDGPU driver reports maximum number of PCIe lanes of Polaris11/Polaris12 as x16
        // in `pp_dpm_pcie` (actually x8), so we use `{current,max}_link_{speed,width}`.
        // ref: drivers/gpu/drm/amd/pm/powerplay/hwmgr/smu7_hwmgr.c
        // 
        // Recent AMD GPUs have multiple endpoints, and the PCIe speed/width actually 
        // runs in that system for the GPU is output to `pp_dpm_pcie`.
        // ref: <https://gitlab.freedesktop.org/drm/amd/-/issues/1967>
        let [current_link, min_dpm_link, max_dpm_link, max_gpu_link, max_system_link] = if is_apu {
            [None; 5]
        } else if vega10_and_later {
            let [min, max] = match pci_bus.get_min_max_link_info_from_dpm() {
                Some([min, max]) => [Some(min), Some(max)],
                None => [None, None],
            };

            [
                pci_bus.get_current_link_info_from_dpm(),
                min,
                max,
                pci_bus.get_max_gpu_link(),
                pci_bus.get_max_system_link(),
            ]
        } else {
            let min = pci_bus.get_min_max_link_info_from_dpm().map(|[min, _]| min);
            let max = pci_bus.get_max_link_info();

            [
                pci_bus.get_current_link_info(),
                min,
                max,
                max,
                pci_bus.get_max_system_link(),
            ]
        };

        let [sclk, mclk, vddnb, vddgfx] = [
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok(),
        ];
        let edge_temp = HwmonTemp::from_hwmon_path(&hwmon_path, HwmonTempType::Edge);
        let junction_temp = HwmonTemp::from_hwmon_path(&hwmon_path, HwmonTempType::Junction);
        let memory_temp = HwmonTemp::from_hwmon_path(&hwmon_path, HwmonTempType::Memory);
        let power_cap = PowerCap::from_hwmon_path(&hwmon_path);
        let average_power = HwmonPower::from_hwmon_path_with_type(&hwmon_path, PowerType::Average);
        let input_power = HwmonPower::from_hwmon_path_with_type(&hwmon_path, PowerType::Input);

        let fan_rpm = parse_hwmon(hwmon_path.join("fan1_input"));
        let fan_max_rpm = parse_hwmon(hwmon_path.join("fan1_max"));
        let gpu_port_path = pci_bus.get_gpu_pcie_port_bus().get_sysfs_path();
        let pci_power_state = std::fs::read_to_string(gpu_port_path.join("power_state")).ok()
            .map(|mut s| {
                s.pop();
                s
            });

        Some(Self {
            hwmon_path,
            is_apu,
            vega10_and_later,
            current_link,
            min_dpm_link,
            max_dpm_link,
            max_gpu_link,
            max_system_link,
            bus_info: *pci_bus,
            sclk,
            mclk,
            vddnb,
            vddgfx,
            edge_temp,
            junction_temp,
            memory_temp,
            average_power,
            input_power,
            power_cap,
            fan_rpm,
            fan_max_rpm,
            gpu_port_path,
            pci_power_state,
        })
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.current_link = if self.is_apu {
            None
        } else if self.vega10_and_later {
            self.bus_info.get_current_link_info_from_dpm()
        } else {
            self.bus_info.get_current_link_info()
        };
        self.sclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok();
        self.mclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok();
        self.vddnb = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok();
        self.vddgfx = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok();

        for temp in [&mut self.edge_temp, &mut self.junction_temp, &mut self.memory_temp] {
            let Some(temp) = temp else { continue };
            temp.update(&self.hwmon_path);
        }

        if self.average_power.is_some() {
            self.average_power = HwmonPower::from_hwmon_path_with_type(
                &self.hwmon_path,
                PowerType::Average,
            );
        }

        if self.input_power.is_some() {
            self.input_power = HwmonPower::from_hwmon_path_with_type(
                &self.hwmon_path,
                PowerType::Input,
            );
        }

        self.fan_rpm = parse_hwmon(self.hwmon_path.join("fan1_input"));
        self.pci_power_state = std::fs::read_to_string(self.gpu_port_path.join("power_state")).ok()
            .map(|mut s| {
                s.pop(); // trim `\n`
                s
            });
    }

    pub fn any_hwmon_power(&self) -> Option<HwmonPower> {
        self.average_power.clone().or(self.input_power.clone())
    }
}
