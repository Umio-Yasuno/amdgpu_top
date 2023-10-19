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
    /* TODO: support both "Average" and "Input" */
    pub hwmon_power: Option<HwmonPower>,
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
                Self::get_max_gpu_link(pci_bus),
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
        let hwmon_power = HwmonPower::from_hwmon_path(&hwmon_path);

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
            hwmon_power,
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

        self.hwmon_power = HwmonPower::from_hwmon_path(&self.hwmon_path);
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

    fn get_max_gpu_link(gpu_pci: &PCI::BUS_INFO) -> Option<PCI::LINK> {
        let mut tmp = Self::get_system_pcie_port_sysfs_path(gpu_pci);

        tmp.pop();

        Self::get_max_link(&tmp)
    }

    fn get_max_system_link(gpu_pci: &PCI::BUS_INFO) -> Option<PCI::LINK> {
        Self::get_max_link(&Self::get_system_pcie_port_sysfs_path(gpu_pci))
    }

    fn get_system_pcie_port_sysfs_path(gpu_pci: &PCI::BUS_INFO) -> PathBuf {
        const NAVI10_UPSTREAM_PORT: &str = "0x1478\n";
        const NAVI10_DOWNSTREAM_PORT: &str = "0x1479\n";

        let mut tmp = gpu_pci.get_sysfs_path().join("../"); // pcie port

        for _ in 0..2 {
            let Ok(did) = std::fs::read_to_string(&tmp.join("device")) else { break };

            if &did == NAVI10_UPSTREAM_PORT || &did == NAVI10_DOWNSTREAM_PORT {
                tmp.push("../");
            } else {
                break;
            }
        }

        tmp
    }

    fn get_max_link(sysfs_path: &PathBuf) -> Option<PCI::LINK> {
        let [s_speed, s_width] = ["max_link_speed", "max_link_width"].map(|name| {
            let mut s = std::fs::read_to_string(sysfs_path.join(name)).ok()?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum PowerType {
    Input,
    Average,
}

impl fmt::Display for PowerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct HwmonPower {
    pub type_: PowerType,
    pub value: u32, // W
}

const POWER1_AVG: &str = "power1_average";
const POWER1_INPUT: &str = "power1_input";

impl HwmonPower {
    pub fn from_hwmon_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let path = path.into();

        let (type_, s) = match std::fs::read_to_string(path.join(POWER1_AVG)) {
            Ok(v) => (PowerType::Average, v),
            Err(_) => {
                let v = std::fs::read_to_string(path.join(POWER1_INPUT)).ok()?;

                (PowerType::Input, v)
            },
        };
        let value = s.trim_end().parse::<u32>().ok()?.saturating_div(1_000_000);

        Some(Self { type_, value })
    }
}
