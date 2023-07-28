use crate::AMDGPU::{
    DeviceHandle,
    drm_amdgpu_info_device,
    drm_amdgpu_memory_info,
    GPU_INFO,
    HwmonTemp,
    IpDieEntry,
    PowerCap,
    VBIOS::VbiosInfo,
    VIDEO_CAPS::{CAP_TYPE, VideoCapsInfo},
};
use crate::{PCI, stat::Sensors};

#[derive(Debug)]
pub struct AppDeviceInfo {
    pub ext_info: drm_amdgpu_info_device,
    pub memory_info: drm_amdgpu_memory_info,
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
    pub pci_bus: PCI::BUS_INFO,
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
    pub ip_die_entries: Vec<IpDieEntry>,
}

impl AppDeviceInfo {
    pub fn new(
        amdgpu_dev: &DeviceHandle,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
        sensors: &Sensors,
    ) -> Self {
        let (min_gpu_clk, max_gpu_clk) = amdgpu_dev.get_min_max_gpu_clock()
            .unwrap_or_else(|| (0, (ext_info.max_engine_clock() / 1000) as u32));
        let (min_mem_clk, max_mem_clk) = amdgpu_dev.get_min_max_memory_clock()
            .unwrap_or_else(|| (0, (ext_info.max_memory_clock() / 1000) as u32));
        let resizable_bar = memory_info.check_resizable_bar();
        let marketing_name = amdgpu_dev.get_marketing_name_or_default();
        let sysfs_path = sensors.bus_info.get_sysfs_path();
        let ip_die_entries = IpDieEntry::get_all_entries_from_sysfs(&sysfs_path);

        Self {
            ext_info: *ext_info,
            memory_info: *memory_info,
            resizable_bar,
            min_dpm_link: sensors.min_dpm_link.clone(),
            max_dpm_link: sensors.max_dpm_link.clone(),
            max_gpu_link: sensors.max_gpu_link.clone(),
            max_system_link: sensors.max_system_link.clone(),
            min_gpu_clk,
            max_gpu_clk,
            min_mem_clk,
            max_mem_clk,
            marketing_name,
            pci_bus: sensors.bus_info,
            edge_temp: sensors.edge_temp.clone(),
            junction_temp: sensors.junction_temp.clone(),
            memory_temp: sensors.memory_temp.clone(),
            power_cap: sensors.power_cap.clone(),
            fan_max_rpm: sensors.fan_max_rpm,
            decode: amdgpu_dev.get_video_caps_info(CAP_TYPE::DECODE).ok(),
            encode: amdgpu_dev.get_video_caps_info(CAP_TYPE::ENCODE).ok(),
            vbios: amdgpu_dev.get_vbios_info().ok(),
            actual_num_tcc_blocks: ext_info.get_actual_num_tcc_blocks(),
            l1_cache_size_kib_per_cu: ext_info.get_l1_cache_size() >> 10,
            gl1_cache_size_kib_per_sa: ext_info.get_gl1_cache_size() >> 10,
            total_l2_cache_size_kib: ext_info.calc_l2_cache_size() >> 10,
            total_l3_cache_size_mib: ext_info.calc_l3_cache_size_mb(),
            ip_die_entries,
        }
    }
}
