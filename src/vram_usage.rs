use libdrm_amdgpu_sys::AMDGPU::drm_amdgpu_memory_info;

pub struct VRAM_USAGE(drm_amdgpu_memory_info);

impl VRAM_USAGE {
    pub fn stat(&self) -> String {
        format!(
            concat!(
                "\n",
                " {vram_label:<5} Usage: {vram_usage:>6}/{vram_total:<6} MiB \n",
                " {gtt_label:<5 } Usage: {gtt_usage:>6}/{gtt_total:<6} MiB \n",
            ),
            vram_label = "VRAM",
            vram_usage = self.0.vram.heap_usage.saturating_div(1024 * 1024),
            vram_total = self.0.vram.total_heap_size.saturating_div(1024 * 1024),
            gtt_label = "GTT",
            gtt_usage = self.0.gtt.heap_usage.saturating_div(1024 * 1024),
            gtt_total = self.0.gtt.total_heap_size.saturating_div(1024 * 1024),
        )
    }
}

impl From<drm_amdgpu_memory_info> for VRAM_USAGE {
    fn from(info: drm_amdgpu_memory_info) -> Self {
        Self(info)
    }
}
