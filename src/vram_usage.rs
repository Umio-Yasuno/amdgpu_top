use libdrm_amdgpu_sys::AMDGPU::drm_amdgpu_memory_info;

pub struct VRAM_INFO {
    pub usable_vram: u64,
    pub usage_vram: u64,
    pub usable_gtt: u64,
    pub usage_gtt: u64,
}

impl VRAM_INFO {
    pub fn stat(&self) -> String {
        format!(
            concat!(
                " {vram_label:<10} => {usage_vram:>6}/{total_vram:<6} MiB \n",
                " {gtt_label:<10 } => {usage_gtt:>6}/{total_gtt:<6} MiB \n",
            ),
            vram_label = "VRAM",
            usage_vram = self.usage_vram.saturating_div(1024 * 1024),
            total_vram = self.usable_vram.saturating_div(1024 * 1024),
            gtt_label = "GTT",
            usage_gtt = self.usage_gtt.saturating_div(1024 * 1024),
            total_gtt = self.usable_gtt.saturating_div(1024 * 1024),
        )
    }
}

impl From<&drm_amdgpu_memory_info> for VRAM_INFO {
    fn from(info: &drm_amdgpu_memory_info) -> Self {
        Self {
            usable_vram: info.vram.usable_heap_size, // (real_vram_size - pin_size - reserved_size)
            usage_vram: info.vram.heap_usage,
            usable_gtt: info.gtt.usable_heap_size,
            usage_gtt: info.gtt.heap_usage,
        }
    }
}
