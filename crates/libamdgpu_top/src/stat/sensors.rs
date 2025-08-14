use std::fmt::{self, Write};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::fs;
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::{
        drm_amdgpu_info_device,
        GPU_INFO,
        ASIC_NAME,
        DeviceHandle,
        DpmClockRange,
        DpmClockType,
        HwmonTemp,
        HwmonTempType,
        SENSOR_INFO::SENSOR_TYPE,
        PowerCap,
        PowerProfile,
    },
};
use super::{CpuFreqInfo, parse_hwmon, HwmonPower, PowerType};

#[derive(Clone, Debug)]
pub struct Sensors {
    pub hwmon_path: PathBuf,
    pub gpu_port_path: PathBuf,
    pub sysfs_path: PathBuf,
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
    pub pci_power_state: Option<String>,
    pub power_profile: Option<PowerProfile>,
    pub fclk_dpm: Option<DpmClockRange>,
    // pub socclk_dpm: Option<DpmClockRange>,
    k10temp_tctl_path: Option<PathBuf>,
    pub tctl: Option<i64>, // CPU Temp.
    pub all_cpu_core_freq_info: Vec<CpuFreqInfo>,
    pub is_idle: bool,
}

impl Sensors {
    pub fn new(
        amdgpu_dev: &DeviceHandle,
        pci_bus: &PCI::BUS_INFO,
        ext_info: &drm_amdgpu_info_device,
    ) -> Option<Self> {
        let hwmon_path = pci_bus.get_hwmon_path()?;
        let sysfs_path = pci_bus.get_sysfs_path();
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
        let pci_power_state = if !is_apu {
            fs::read_to_string(gpu_port_path.join("power_state"))
                .ok()
                .map(|mut s| {
                    s.pop();
                    s
                })
        } else {
            None
        };
        let power_profile = PowerProfile::get_current_profile_from_sysfs(&sysfs_path);
        let k10temp_path = if is_apu {
            Self::find_k10temp_path()
        } else {
            None
        };
        let k10temp_tctl_path = k10temp_path.map(|path| path.join("temp1_input"));
        let tctl = if let Some(ref path) = k10temp_tctl_path {
            Self::get_tctl(path)
        } else {
            None
        };

        let all_cpu_core_freq_info = if is_apu {
            CpuFreqInfo::get_all_cpu_core_freq_info()
        } else {
            Vec::new()
        };

        let fclk_dpm = DpmClockRange::from_sysfs(DpmClockType::FCLK, &sysfs_path);
        // let socclk_dpm = DpmClockRange::from_sysfs(DpmClockType::SOCCLK, &sysfs_path);

        Some(Self {
            hwmon_path,
            sysfs_path,
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
            power_profile,
            fclk_dpm,
            // socclk_dpm,
            k10temp_tctl_path,
            tctl,
            all_cpu_core_freq_info,
            is_idle: false,
        })
    }

    pub fn update_without_device_handle(&mut self) {
        self.current_link = if self.is_apu {
            None
        } else if self.vega10_and_later {
            self.bus_info.get_current_link_info_from_dpm()
        } else {
            self.bus_info.get_current_link_info()
        };

        let hwmon_path = &self.hwmon_path;

        self.edge_temp = HwmonTemp::from_hwmon_path(hwmon_path, HwmonTempType::Edge);
        self.junction_temp = HwmonTemp::from_hwmon_path(hwmon_path, HwmonTempType::Junction);
        self.memory_temp = HwmonTemp::from_hwmon_path(hwmon_path, HwmonTempType::Memory);
        self.power_cap = PowerCap::from_hwmon_path(hwmon_path);
        self.average_power = HwmonPower::from_hwmon_path_with_type(hwmon_path, PowerType::Average);
        self.input_power = HwmonPower::from_hwmon_path_with_type(hwmon_path, PowerType::Input);

        self.fan_rpm = parse_hwmon(self.hwmon_path.join("fan1_input"));
        self.power_profile = PowerProfile::get_current_profile_from_sysfs(&self.sysfs_path);
        self.power_cap = PowerCap::from_hwmon_path(&self.hwmon_path);
        self.update_pci_power_state();
        self.update_tctl();
        self.update_all_cpu_core_cur_freq();
        self.is_idle = false;
    }

    pub fn update_for_idle(&mut self) {
        self.current_link = None;
        self.edge_temp = None;
        self.junction_temp = None;
        self.memory_temp = None;
        self.average_power = None;
        self.input_power = None;
        self.sclk = None;
        self.mclk = None;
        self.vddnb = None;
        self.vddgfx = None;
        self.fan_rpm = None;
        self.power_profile = None;
        self.fclk_dpm = None;
        self.is_idle = true;

        self.update_pci_power_state();
    }

    pub fn update_pci_power_state(&mut self) {
        if !self.is_apu {
            self.pci_power_state = fs::read_to_string(self.gpu_port_path.join("power_state"))
                .ok()
                .map(|mut s| {
                    s.pop(); // trim `\n`
                    s
                });
        }
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.update_without_device_handle();
        self.sclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok();
        self.mclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok();
        self.vddnb = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok();
        self.vddgfx = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok();
        self.fclk_dpm = DpmClockRange::from_sysfs(DpmClockType::FCLK, &self.sysfs_path);
        // self.socclk_dpm = DpmClockRange::from_sysfs(DpmClockType::SOCCLK, &self.sysfs_path);
    }

    pub fn any_hwmon_power(&self) -> Option<HwmonPower> {
        self.average_power.clone().or(self.input_power.clone())
    }

    fn find_k10temp_path() -> Option<PathBuf> {
        const HWMON_DIR: &str = "/sys/class/hwmon/";
        const K10TEMP_NAME: &[u8] = b"k10temp";
        let hwmon_dir = fs::read_dir(HWMON_DIR).ok()?;
        let mut buf = [0u8; 8];

        for dir_entry in hwmon_dir {
            let Ok(dir_entry)= dir_entry else { continue };
            let path = dir_entry.path();

            let Ok(mut f) = fs::File::open(path.join("name")) else { continue };
            let _ = f.read_exact(&mut buf);

            if buf.starts_with(K10TEMP_NAME) {
                return Some(path);
            }
        }

        None
    }

    fn get_tctl(tctl_path: &Path) -> Option<i64> {
        let val = fs::read_to_string(tctl_path).ok()?;

        val.trim_end().parse::<i64>().ok()
    }

    fn update_tctl(&mut self) {
        if let Some(ref path) = self.k10temp_tctl_path {
            self.tctl = Self::get_tctl(path);
        }
    }

    fn update_all_cpu_core_cur_freq(&mut self) {
        for freq_info in self.all_cpu_core_freq_info.iter_mut() {
            freq_info.update_cur_freq();
        }
    }

    pub fn print_all_cpu_core_cur_freq(
        &self,
        buf: &mut String,
        label: &str,
        divide_by_100: bool,
    ) -> fmt::Result {
        if self.all_cpu_core_freq_info.is_empty() { return Ok(()) }

        write!(buf, "{label}: [")?;

        for cpu_freq_info in &self.all_cpu_core_freq_info {
            if divide_by_100 {
                write!(
                    buf,
                    "{:>2},",
                    cpu_freq_info.cur.div_ceil(100),
                )?;
            } else {
                write!(
                    buf,
                    "{:>4},",
                    cpu_freq_info.cur,
                )?;
            }
        }

        let _ = buf.pop();
        write!(buf, "]")?;

        Ok(())
    }
}
