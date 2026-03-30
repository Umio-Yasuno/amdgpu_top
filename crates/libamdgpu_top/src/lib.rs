use std::time::Duration;
use std::path::PathBuf;

use num_enum::{IntoPrimitive, TryFromPrimitive};

pub use libdrm_amdgpu_sys::*;
use libdrm_amdgpu_sys::AMDGPU::{
    CHIP_CLASS,
    DeviceHandle,
    drm_amdgpu_memory_info,
    HW_IP::{HW_IP_TYPE, HwIpInfo},
    GpuMetrics, MetricsInfo,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum GuiMode {
    Auto,
    Single,
    Tab,
}

impl GuiMode {
    pub fn is_tab_mode(&self) -> bool {
        *self == Self::Tab
    }
}

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
    pub gui_wgpu_backend: GuiWgpuBackend, // GUI
    pub gui_mode: GuiMode, // GUI
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
    let rocm_path = PathBuf::from(rocm_path);

    let s = std::fs::read_to_string(rocm_path.join(".info/version")).ok()?;
    let s = s.trim_end();

    if let Some((ver, _)) = s.split_once('-') {
        Some(ver.to_string())
    } else {
        Some(s.to_string())
    }
}

// ref: drivers/platform/x86/amd/pmf/pmf.h
pub struct NpuMetrics {
    pub npuclk_freq: u16,
    pub npu_busy: Vec<u16>,
    pub npu_power: u16,
    pub mpnpuclk_freq: u16,
    pub npu_reads: u16,
    pub npu_writes: u16,
}

pub trait GetNpuMetrics {
    fn get_npu_metrics(&self) -> Option<NpuMetrics>;
}

impl GetNpuMetrics for GpuMetrics {
    fn get_npu_metrics(&self) -> Option<NpuMetrics> {
        let npuclk_freq = self.get_average_ipuclk_frequency()?;
        let npu_busy = self.get_average_ipu_activity()?;
        let npu_power = self.get_average_ipu_power()?;
        let mpnpuclk_freq = self.get_average_mpipu_frequency()?;
        let npu_reads = self.get_average_ipu_reads()?;
        let npu_writes = self.get_average_ipu_writes()?;

        Some(NpuMetrics {
            npuclk_freq,
            npu_busy,
            npu_power,
            mpnpuclk_freq,
            npu_reads,
            npu_writes,
        })
    }
}
