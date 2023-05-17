use libamdgpu_top::AMDGPU::{
    DeviceHandle,
    drm_amdgpu_info_device,
    drm_amdgpu_memory_info,
    HW_IP::{HwIpInfo, HW_IP_TYPE},
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
    pub critical_temp: Option<u32>,
    pub power_cap: Option<u32>,
    pub power_cap_min: Option<u32>,
    pub power_cap_max: Option<u32>,
    pub fan_max_rpm: Option<u32>,
}

impl AppDeviceInfo {
    pub fn new(
        amdgpu_dev: &DeviceHandle,
        ext_info: &drm_amdgpu_info_device,
        memory_info: &drm_amdgpu_memory_info,
        sensors: &Sensors,
    ) -> Self {
        let (min_gpu_clk, max_gpu_clk) =
            amdgpu_dev.get_min_max_gpu_clock().unwrap_or((0, 0));
        let (min_mem_clk, max_mem_clk) =
            amdgpu_dev.get_min_max_memory_clock().unwrap_or((0, 0));
        let resizable_bar = memory_info.check_resizable_bar();
        let marketing_name = amdgpu_dev.get_marketing_name().unwrap_or_default();
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
            critical_temp: sensors.critical_temp,
            power_cap: sensors.power_cap,
            power_cap_min: sensors.power_cap_min,
            power_cap_max: sensors.power_cap_max,
            fan_max_rpm: sensors.fan_max_rpm,
        }
    }
}
