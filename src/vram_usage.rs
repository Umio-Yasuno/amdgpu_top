use libdrm_amdgpu_sys::AMDGPU::drm_amdgpu_memory_info;

pub struct VRAM_INFO {
    pub total_vram: u64,
    pub usable_vram: u64,
    pub usage_vram: u64,
    pub total_gtt: u64,
    pub usable_gtt: u64,
    pub usage_gtt: u64,
}

impl VRAM_INFO {
    pub fn stat(&self) -> String {
        format!(
            concat!(
                " {vram_label:<5} => {usage_vram:^5}/{total_vram:^5} MiB,",
                " {gtt_label:<5 } => {usage_gtt:^5}/{total_gtt:^5} MiB",
            ),
            vram_label = "VRAM",
            usage_vram = self.usage_vram >> 20,
            total_vram = self.total_vram >> 20,
            gtt_label = "GTT",
            usage_gtt = self.usage_gtt >> 20,
            total_gtt = self.total_gtt >> 20,
        )
    }
}

impl From<&drm_amdgpu_memory_info> for VRAM_INFO {
    fn from(info: &drm_amdgpu_memory_info) -> Self {
        // usable_heap_size is not fixed.
        // usable_heap_size = real_vram_size - pin_size - reserved_size
        Self {
            total_vram: info.vram.total_heap_size,
            usable_vram: info.vram.usable_heap_size, 
            usage_vram: info.vram.heap_usage,
            total_gtt: info.gtt.total_heap_size,
            usable_gtt: info.gtt.usable_heap_size,
            usage_gtt: info.gtt.heap_usage,
        }
    }
}
