use libdrm_amdgpu_sys::{
    AMDGPU::{drm_amdgpu_info_device, DeviceHandle, GPU_INFO},
};

pub fn info_bar(amdgpu_dev: &DeviceHandle, ext_info: &drm_amdgpu_info_device) -> String {
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let chip_class = ext_info.get_chip_class();

    let (min_gpu_clk, max_gpu_clk) = amdgpu_dev.get_min_max_gpu_clock().unwrap_or((0, 0));
    let (min_mem_clk, max_mem_clk) = amdgpu_dev.get_min_max_memory_clock().unwrap_or((0, 0));
    let mark_name = amdgpu_dev.get_marketing_name().unwrap_or("".to_string());

    format!(
        concat!(
            "{mark_name} ({did:#06X}:{rid:#04X})\n",
            "{asic}, {gpu_type}, {chip_class}, {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
            "{vram_type} {vram_bus_width}-bit, {vram_size} MiB, ",
            "{min_memory_clk}-{max_memory_clk} MHz",
        ),
        mark_name = mark_name,
        did = ext_info.device_id(),
        rid = ext_info.pci_rev_id(),
        asic = ext_info.get_asic_name(),
        gpu_type = if ext_info.is_apu() { "APU" } else { "dGPU" },
        chip_class = chip_class,
        num_cu = ext_info.cu_active_number(),
        min_gpu_clk = min_gpu_clk,
        max_gpu_clk = max_gpu_clk,
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
        vram_size = memory_info.vram.total_heap_size >> 20,
        min_memory_clk = min_mem_clk,
        max_memory_clk = max_mem_clk,
    )
}
