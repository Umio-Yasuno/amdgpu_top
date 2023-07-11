use std::fmt::{self, Write};
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
use super::parse_hwmon;

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
    pub power: Option<u32>,
    pub power_cap: Option<PowerCap>,
    pub fan_rpm: Option<u32>,
    pub fan_max_rpm: Option<u32>,
}

impl Sensors {
    pub fn new(
        amdgpu_dev: &DeviceHandle,
        pci_bus: &PCI::BUS_INFO,
        ext_info: &drm_amdgpu_info_device,
    ) -> Self {
        let hwmon_path = pci_bus.get_hwmon_path().unwrap();
        let asic_name = ext_info.get_asic_name();
        let is_apu = ext_info.is_apu();
        let vega10_and_later = ASIC_NAME::CHIP_VEGA10 <= asic_name;

        // AMDGPU driver reports maximum number of PCIe lanes of Polaris11/Polaris12 as x16
        // in `pp_dpm_pcie` (actually x8), so we use `{current,max}_link_{speed,width}`.
        // ref: drivers/gpu/drm/amd/pm/powerplay/hwmgr/smu7_hwmgr.c
        // 
        // However, recent AMD GPUs have multiple endpoints, and the PCIe speed/width actually 
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
                Some(pci_bus.get_link_info(PCI::STATUS::Max)),
                Self::get_max_system_link(pci_bus),
            ]
        } else {
            let min = match pci_bus.get_min_max_link_info_from_dpm() {
                Some([min, _]) => Some(min),
                None => None,
            };
            let max = pci_bus.get_link_info(PCI::STATUS::Max);

            [
                Some(pci_bus.get_link_info(PCI::STATUS::Current)),
                min,
                Some(max.clone()),
                Some(max.clone()),
                Self::get_max_system_link(pci_bus),
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
            power,
            power_cap,
            fan_rpm,
            fan_max_rpm,
        }
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.current_link = if self.is_apu {
            None
        } else if self.vega10_and_later {
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

    pub fn print_pcie_link(&self) -> Result<String, fmt::Error> {
        let mut buf = String::new();

        if let Some(cur) = self.current_link {
            write!(buf, "PCIe Link Speed => Gen{}x{:<2}", cur.gen, cur.width)?;
        } else {
            return Ok(buf);
        }

        if let [Some(min), Some(max)] = [self.min_dpm_link, self.max_dpm_link] {
            write!(
                buf,
                " (Gen{}x{} - Gen{}x{})",
                min.gen,
                min.width,
                max.gen,
                max.width,
            )?;
        } else if let Some(max) = self.max_dpm_link {
            write!(buf, " (Max. Gen{}x{})", max.gen, max.width)?;
        }

        Ok(buf)
    }

    fn get_max_system_link(gpu_pci: &PCI::BUS_INFO) -> Option<PCI::LINK> {
        let base_path = gpu_pci.get_sysfs_path().join("../"); // system pcie port
        let [s_speed, s_width] = ["max_link_speed", "max_link_width"].map(|name| {
            let mut s = std::fs::read_to_string(base_path.join(name)).ok()?;
            s.pop(); // trim `\n`

            Some(s)
        });

        let gen = match s_speed?.as_str() {
            "2.5 GT/s PCIe" => 1,
            "5.0 GT/s PCIe" => 2,
            "8.0 GT/s PCIe" => 3,
            "16.0 GT/s PCIe" => 4,
            "32.0 GT/s PCIe" => 5,
            "64.0 GT/s PCIe" => 6,
            _ => 0,
        };
        let width = s_width?.parse::<u8>().ok()?;

        Some(PCI::LINK { gen, width })
    }
}
