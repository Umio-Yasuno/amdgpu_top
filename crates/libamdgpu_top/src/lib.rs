use std::time::Duration;
pub use libdrm_amdgpu_sys::*;
use libdrm_amdgpu_sys::AMDGPU::{
    DeviceHandle,
    drm_amdgpu_memory_info,
    HW_IP::{HW_IP_TYPE, HwIpInfo},
};

mod app_device_info;
pub use app_device_info::*;

pub mod stat;
pub mod app;

mod device_path;
pub use device_path::DevicePath;

mod drm_mode;
pub use drm_mode::*;

mod ppfeaturemask;
pub use ppfeaturemask::*;

pub struct Sampling {
    pub count: usize,
    pub delay: Duration,
}

impl Default for Sampling {
    fn default() -> Self {
        Self::low()
    }
}

impl Sampling {
    pub const fn low() -> Self {
        Self {
            count: 100,
            delay: Duration::from_millis(10),
        }
    }

    pub const fn high() -> Self {
        Self {
            count: 100,
            delay: Duration::from_millis(1),
        }
    }

    pub fn to_duration(&self) -> Duration {
        self.delay * self.count as u32
    }
}

#[derive(Debug, Clone)]
pub struct VramUsage(pub drm_amdgpu_memory_info);

impl VramUsage {
    pub fn new(memory_info: &drm_amdgpu_memory_info) -> Self {
        Self(*memory_info)
    }

    pub fn update_usage(&mut self, amdgpu_dev: &DeviceHandle) {
        if let [Ok(vram), Ok(vis_vram), Ok(gtt)] = [
            amdgpu_dev.vram_usage_info(),
            amdgpu_dev.vis_vram_usage_info(),
            amdgpu_dev.gtt_usage_info(),
        ] {
            self.0.vram.heap_usage = vram;
            self.0.cpu_accessible_vram.heap_usage = vis_vram;
            self.0.gtt.heap_usage = gtt;
        }
    }

    pub fn update_usable_heap_size(&mut self, amdgpu_dev: &DeviceHandle) {
        let Ok(info) = amdgpu_dev.vram_gtt_info() else { return };

        self.0.vram.usable_heap_size = info.vram_size;
        self.0.cpu_accessible_vram.usable_heap_size = info.vram_cpu_accessible_size;
        self.0.gtt.usable_heap_size = info.gtt_size;
    }
}

pub fn has_vcn(amdgpu_dev: &DeviceHandle) -> bool {
    amdgpu_dev.get_hw_ip_info(HW_IP_TYPE::VCN_DEC).is_ok()
}

pub fn has_vcn_unified(amdgpu_dev: &DeviceHandle) -> bool {
    let Ok(ip) = amdgpu_dev.get_hw_ip_info(HW_IP_TYPE::VCN_ENC) else { return false };

    4 <= ip.info.hw_ip_version_major
}

pub fn get_hw_ip_info_list(amdgpu_dev: &DeviceHandle) -> Vec<HwIpInfo> {
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

    HW_IP_LIST.iter()
        .filter_map(|ip_type| amdgpu_dev.get_hw_ip_info(*ip_type).ok())
        .filter(|hw_ip_info| hw_ip_info.count != 0).collect()
}
