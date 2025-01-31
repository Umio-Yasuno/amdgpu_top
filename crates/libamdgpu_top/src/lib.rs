use std::time::Duration;
use std::path::PathBuf;
pub use libdrm_amdgpu_sys::*;
use libdrm_amdgpu_sys::AMDGPU::{
    CHIP_CLASS,
    DeviceHandle,
    drm_amdgpu_memory_info,
    HW_IP::{HW_IP_TYPE, HwIpInfo},
};

mod app_device_info;
pub use app_device_info::*;

pub mod stat;
pub mod app;
pub mod xdna;

mod device_path;
pub use device_path::{DeviceType, DevicePath};

mod drm_mode;
pub use drm_mode::*;

mod ppfeaturemask;
pub use ppfeaturemask::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GuiWgpuBackend {
    Gl,
    Vulkan,
}

#[derive(Debug, Clone)]
pub struct UiArgs {
    pub selected_device_path: DevicePath,
    pub device_path_list: Vec<DevicePath>,
    pub update_process_index: u64,
    pub no_pc: bool,
    pub is_dark_mode: Option<bool>, // TUI, GUI
    pub hide_fdinfo: bool, // TUI
    pub gui_wgpu_backend: GuiWgpuBackend,
}

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

pub fn has_vpe(amdgpu_dev: &DeviceHandle) -> bool {
    amdgpu_dev.get_hw_ip_info(HW_IP_TYPE::VPE).is_ok()
}

pub fn get_hw_ip_info_list(
    amdgpu_dev: &DeviceHandle,
    chip_class: CHIP_CLASS,
) -> Vec<HwIpInfo> {
    const HW_IP_LIST: &[HW_IP_TYPE] = &[
        // HW_IP_TYPE::GFX,
        // HW_IP_TYPE::COMPUTE,
        HW_IP_TYPE::DMA,
        HW_IP_TYPE::UVD,
        HW_IP_TYPE::VCE,
        HW_IP_TYPE::UVD_ENC,
        HW_IP_TYPE::VCN_DEC,
        HW_IP_TYPE::VCN_ENC,
        HW_IP_TYPE::VCN_JPEG,
        HW_IP_TYPE::VPE,
    ];

    let mut hw_ip_list: Vec<HwIpInfo> = Vec::with_capacity(10);

    {
        for ip_type in [HW_IP_TYPE::GFX, HW_IP_TYPE::COMPUTE] {
            let Ok(mut ip_info) = amdgpu_dev.get_hw_ip_info(ip_type) else { continue };

            // Fix incorrect IP versions reported by the kernel.
            // ref: https://gitlab.freedesktop.org/mesa/mesa/blob/main/src/amd/common/ac_gpu_info.c
            match chip_class {
                CHIP_CLASS::GFX10 => ip_info.info.hw_ip_version_minor = 1,
                CHIP_CLASS::GFX10_3 => ip_info.info.hw_ip_version_minor = 3,
                _ => {},
            }

            if ip_info.count != 0 {
                hw_ip_list.push(ip_info);
            }
        }
    }

    for ip_type in HW_IP_LIST {
        let Ok(ip_info) = amdgpu_dev.get_hw_ip_info(*ip_type) else { continue };

        if ip_info.count != 0 {
            hw_ip_list.push(ip_info);
        }
    }

    hw_ip_list
}

pub fn get_rocm_version() -> Option<String> {
    let rocm_path = std::env::var("ROCM_PATH").unwrap_or("/opt/rocm".to_string());
    let s = std::fs::read_to_string(PathBuf::from(rocm_path).join(".info/version")).ok()?;

    s.split_once('-').map(|(ver, _)| ver.to_string())
}
