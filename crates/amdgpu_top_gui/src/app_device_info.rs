use libamdgpu_top::AMDGPU::{
    DeviceHandle,
    drm_amdgpu_info_device,
    drm_amdgpu_memory_info,
    GPU_INFO,
    HW_IP::{HwIpInfo, HW_IP_TYPE},
    HwmonTemp,
    PowerCap,
};
use libamdgpu_top::AMDGPU::{
    VBIOS::VbiosInfo,
    VIDEO_CAPS::{CAP_TYPE, VideoCapsInfo},
};
use libamdgpu_top::{PCI, stat::Sensors};

const HW_IP_LIST: &[HW_IP_TYPE] = &[
    HW_IP_TYPE::GFX,
    HW_IP_TYPE::COMPUTE,
    HW_IP_TYPE::DMA,
    HW_IP_TYPE::UVD,
    HW_IP_TYPE::VCE,
    HW_IP_TYPE::UVD_ENC,
    HW_IP_TYPE::VCN_DEC,
    HW_IP_TYPE::VCN_ENC,
    HW_IP_TYPE::VCN_JPEG,
];

#[derive(Clone)]
pub struct AppDeviceInfo {
    pub ext_info: drm_amdgpu_info_device,
    pub memory_info: drm_amdgpu_memory_info,
    pub hw_ip_info: Vec<HwIpInfo>,
    pub resizable_bar: bool,
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
        let hw_ip_info = HW_IP_LIST.iter()
            .filter_map(|ip_type| amdgpu_dev.get_hw_ip_info(*ip_type).ok())
            .filter(|hw_ip_info| hw_ip_info.count != 0).collect();

        Self {
            ext_info: *ext_info,
            memory_info: *memory_info,
            hw_ip_info,
            resizable_bar,
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
        }
    }
}
