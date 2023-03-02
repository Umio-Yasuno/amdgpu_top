use crate::AMDGPU::drm_amdgpu_memory_info;

pub struct VRAM_USAGE(drm_amdgpu_memory_info);

impl VRAM_USAGE {
    pub fn stat(&self) -> String {
        format!(
            concat!(
                "VRAM Usage: {vram_usage:>6}/{vram_total:<6} MiB\n",
                "GTT  Usage: {gtt_usage:>6}/{gtt_total:<6} MiB\n",
            ),
            vram_usage = self.0.vram.heap_usage.saturating_div(1024 * 1024),
            vram_total = self.0.vram.total_heap_size.saturating_div(1024 * 1024),
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
