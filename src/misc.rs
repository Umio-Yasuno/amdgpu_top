use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::DeviceHandle,
};

pub fn get_min_clk(
    amdgpu_dev: &DeviceHandle,
    pci_bus: &PCI::BUS_INFO
) -> (u64, u64) {
    (
        amdgpu_dev.get_min_gpu_clock_from_sysfs(pci_bus).unwrap_or_else(|| 0),
        amdgpu_dev.get_min_memory_clock_from_sysfs(pci_bus).unwrap_or_else(|| 0),
    )
}
