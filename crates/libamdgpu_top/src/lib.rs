use std::time::Duration;
pub use libdrm_amdgpu_sys::*;
use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, drm_amdgpu_memory_info};

pub mod stat;

mod device_path;
pub use device_path::DevicePath;

pub struct Sampling {
    pub count: usize,
    pub delay: Duration,
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
}
