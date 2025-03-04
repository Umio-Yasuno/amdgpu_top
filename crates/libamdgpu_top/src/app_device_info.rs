use crate::AMDGPU::{
    ASIC_NAME,
    DeviceHandle,
    drm_amdgpu_info_device,
    drm_amdgpu_memory_info,
    FW_VERSION::FW_TYPE,
    GPU_INFO,
    HW_IP::HwIpInfo,
    HwmonTemp,
    IpDieEntry,
    PowerCap,
    PowerProfile,
    RasBlock,
    RasErrorCount,
    VBIOS::VbiosInfo,
    VIDEO_CAPS::{CAP_TYPE, VideoCapsInfo},
};
use crate::{DevicePath, get_hw_ip_info_list, PCI, stat::Sensors};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppDeviceInfo {
    pub ext_info: drm_amdgpu_info_device,
    pub memory_info: drm_amdgpu_memory_info,
    pub is_apu: bool,
    pub resizable_bar: bool,
    pub min_dpm_link: Option<PCI::LINK>,
    pub max_dpm_link: Option<PCI::LINK>,
    pub max_gpu_link: Option<PCI::LINK>,
    pub max_system_link: Option<PCI::LINK>,
    pub min_gpu_clk: u32,
    pub max_gpu_clk: u32,
    pub min_mem_clk: u32,
    pub max_mem_clk: u32,
    pub marketing_name: String,
    pub asic_name: ASIC_NAME,
    pub pci_bus: PCI::BUS_INFO,
    pub sysfs_path: PathBuf,
    pub edge_temp: Option<HwmonTemp>,
    pub junction_temp: Option<HwmonTemp>,
    pub memory_temp: Option<HwmonTemp>,
    pub power_cap: Option<PowerCap>,
    pub fan_max_rpm: Option<u32>,
    pub decode: Option<VideoCapsInfo>,
    pub encode: Option<VideoCapsInfo>,
    pub vbios: Option<VbiosInfo>,
    pub l1_cache_size_kib_per_cu: u32,
    pub actual_num_tcc_blocks: u32,
    pub gl1_cache_size_kib_per_sa: u32,
    pub total_l2_cache_size_kib: u32,
    pub total_l3_cache_size_mib: u32,
    pub hw_ip_info_list: Vec<HwIpInfo>,
    pub ip_die_entries: Vec<IpDieEntry>,
    pub power_profiles: Vec<PowerProfile>,
    pub gfx_target_version: Option<String>,
    pub ecc_memory: bool,
    pub has_npu: bool,
    pub smc_fw_version: Option<u32>,
}

impl AppDeviceInfo {
    pub fn new(
        amdgpu_dev: &DeviceHandle,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
        sensors: &Option<Sensors>,
        device_path: &DevicePath,
    ) -> Self {
        let (min_gpu_clk, max_gpu_clk) = amdgpu_dev.get_min_max_gpu_clock()
            .unwrap_or_else(|| (0, (ext_info.max_engine_clock() / 1000) as u32));
        let (min_mem_clk, max_mem_clk) = amdgpu_dev.get_min_max_memory_clock()
            .unwrap_or_else(|| (0, (ext_info.max_memory_clock() / 1000) as u32));
        let resizable_bar = memory_info.check_resizable_bar();
        let is_apu = ext_info.is_apu();
        let marketing_name = device_path.device_name.clone();
        let sysfs_path = device_path.sysfs_path.clone();
        let hw_ip_info_list = get_hw_ip_info_list(amdgpu_dev, ext_info.get_chip_class());
        let ip_die_entries = IpDieEntry::get_all_entries_from_sysfs(&sysfs_path);
        let power_profiles = PowerProfile::get_all_supported_profiles_from_sysfs(&sysfs_path);
        let asic_name = ext_info.get_asic_name();
        let gfx_target_version = ext_info.get_gfx_target_version().map(|v| v.to_string());

        let ecc_memory = RasErrorCount::get_from_sysfs_with_ras_block(&sysfs_path, RasBlock::UMC).is_ok();
        let has_npu = is_apu && match asic_name {
            ASIC_NAME::CHIP_GFX1103_R1 |
            ASIC_NAME::CHIP_GFX1103_R1X => true,
            _ => asic_name >= ASIC_NAME::CHIP_GFX1150,
        };
        let smc_fw_version = amdgpu_dev
            .query_firmware_version(FW_TYPE::SMC, 0, 0)
            .ok()
            .map(|fw_ver| fw_ver.version);

        Self {
            ext_info: *ext_info,
            memory_info: *memory_info,
            resizable_bar,
            is_apu,
            min_dpm_link: sensors.as_ref().and_then(|s| s.min_dpm_link),
            max_dpm_link: sensors.as_ref().and_then(|s| s.max_dpm_link),
            max_gpu_link: sensors.as_ref().and_then(|s| s.max_gpu_link),
            max_system_link: sensors.as_ref().and_then(|s| s.max_system_link),
            min_gpu_clk,
            max_gpu_clk,
            min_mem_clk,
            max_mem_clk,
            marketing_name,
            asic_name,
            pci_bus: device_path.pci,
            sysfs_path,
            edge_temp: sensors.as_ref().and_then(|s| s.edge_temp.clone()),
            junction_temp: sensors.as_ref().and_then(|s| s.junction_temp.clone()),
            memory_temp: sensors.as_ref().and_then(|s| s.memory_temp.clone()),
            power_cap: sensors.as_ref().and_then(|s| s.power_cap.clone()),
            fan_max_rpm: sensors.as_ref().and_then(|s| s.fan_max_rpm),
            decode: amdgpu_dev.get_video_caps_info(CAP_TYPE::DECODE).ok(),
            encode: amdgpu_dev.get_video_caps_info(CAP_TYPE::ENCODE).ok(),
            vbios: amdgpu_dev.get_vbios_info().ok(),
            actual_num_tcc_blocks: ext_info.get_actual_num_tcc_blocks(),
            l1_cache_size_kib_per_cu: ext_info.get_l1_cache_size() >> 10,
            gl1_cache_size_kib_per_sa: ext_info.get_gl1_cache_size() >> 10,
            total_l2_cache_size_kib: ext_info.calc_l2_cache_size() >> 10,
            total_l3_cache_size_mib: ext_info.calc_l3_cache_size_mb(),
            hw_ip_info_list,
            ip_die_entries,
            power_profiles,
            gfx_target_version,
            ecc_memory,
            has_npu,
            smc_fw_version,
        }
    }

    pub fn menu_entry(&self) -> String {
        format!("{} ({})", self.marketing_name, self.pci_bus)
    }
}
