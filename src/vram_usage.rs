use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, drm_amdgpu_memory_info};

#[allow(non_camel_case_types)]
pub struct VRAM_INFO {
    pub total_vram: u64,
    pub usable_vram: u64,
    pub usage_vram: u64,
    pub total_gtt: u64,
    pub usable_gtt: u64,
    pub usage_gtt: u64,
    pub buf: String,
}

impl VRAM_INFO {
    pub fn update_usage(&mut self, amdgpu_dev: &DeviceHandle) {
        if let [Ok(usage_vram), Ok(usage_gtt)] = [
            amdgpu_dev.vram_usage_info(),
            amdgpu_dev.gtt_usage_info(),
        ] {
            self.usage_vram = usage_vram;
            self.usage_gtt = usage_gtt;
        }
    }

    pub fn print(&mut self) {
        use std::fmt::Write;

        self.buf.clear();

        write!(
            self.buf,
            concat!(
                " {vram_label:<5} => {usage_vram:>5}/{total_vram:<5} MiB,",
                " {gtt_label:>17 } => {usage_gtt:>5}/{total_gtt:<5} MiB ",
            ),
            vram_label = "VRAM",
            usage_vram = self.usage_vram >> 20,
            total_vram = self.total_vram >> 20,
            gtt_label = "GTT",
            usage_gtt = self.usage_gtt >> 20,
            total_gtt = self.total_gtt >> 20,
        )
        .unwrap();
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
            buf: String::new(),
        }
    }
}
