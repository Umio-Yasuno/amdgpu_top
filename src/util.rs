use libdrm_amdgpu_sys::*;
// use AMDGPU::GPU_INFO;

pub(crate) fn get_min_clk(
    amdgpu_dev: &AMDGPU::DeviceHandle,
    pci_bus: &PCI::BUS_INFO
) -> (u64, u64) {
    if let [Some(gpu), Some(mem)] = [
        amdgpu_dev.get_min_gpu_clock_from_sysfs(&pci_bus),
        amdgpu_dev.get_min_memory_clock_from_sysfs(&pci_bus),
    ] {
        (gpu, mem)
    } else {
        (0, 0)
    }
}

pub(crate) fn check_register_offset(
    amdgpu_dev: &AMDGPU::DeviceHandle,
    name: &str,
    offset: u32
) -> bool {
    if let Err(err) = amdgpu_dev.read_mm_registers(offset) {
        println!("{name} ({offset:#X}) register is not allowed. ({err})");
        return false;
    }

    true
}

pub(crate) fn vbios_info(amdgpu_dev: &AMDGPU::DeviceHandle) {
    if let Ok(vbios) = unsafe { amdgpu_dev.vbios_info() } {
        let [name, pn, ver_str, date] = [
            vbios.name.to_vec(),
            vbios.vbios_pn.to_vec(),
            vbios.vbios_ver_str.to_vec(),
            vbios.date.to_vec(),
        ]
        .map(|v| {
            let tmp = String::from_utf8(v).unwrap();

            tmp.trim_end_matches(|c: char| c.is_control() || c.is_whitespace()).to_string()
        });

        println!("\nVBIOS info:");
        println!("name:\t[{name}]");
        println!("pn:\t[{pn}]");
        println!("ver_str:[{ver_str}]");
        println!("date:\t[{date}]");
    }
}
