use libamdgpu_top::{
    AMDGPU::{
        self,
        GPU_INFO,
        HwId,
        IpHwId,
        PPTable,
    },
    DevicePath,
};

pub fn dump_all_pp_table(title: &str, device_path_list: &[DevicePath]) {
    println!("{title}\n");

    for device_path in device_path_list {
        dump_pp_table(device_path);
    }
}

fn dump_pp_table(device_path: &DevicePath) {
    let Ok(amdgpu_dev) = device_path.init() else { return };

    {
        let Ok(ext_info) = amdgpu_dev.device_info() else { return };
        let mark_name = ext_info.find_device_name_or_default();
        println!("{mark_name} ({}, {:#0X}:{:#0X})", device_path.pci, ext_info.device_id(), ext_info.pci_rev_id());
    }

    let sysfs = device_path.pci.get_sysfs_path();
    let smu = IpHwId::get_from_die_id_sysfs(HwId::MP1, &sysfs.join("ip_discovery/die/0/")).ok().and_then(|smu| smu.instances.get(0).map(|v| v.clone()));

    if let Some(smu) = &smu {
        println!("    SMU (MP1) version: {}.{}.{}", smu.major, smu.minor, smu.revision);
    }

    let pp_table_bytes_sysfs = std::fs::read(&sysfs.join("pp_table")).ok();
    let pp_table_bytes_vbios = amdgpu_dev.get_vbios_image().ok().and_then(|vbios_image| {
        use AMDGPU::VBIOS::VbiosParser;

        let vbios_parser = VbiosParser::new(vbios_image);
        let rom_header = vbios_parser.get_atom_rom_header()?;
        let data_table = vbios_parser.get_atom_data_table(&rom_header)?;

        Some(vbios_parser.get_powerplay_table_bytes(&data_table)?.to_vec())
    });

    for (bytes, src) in [
        (pp_table_bytes_sysfs, "sysfs"),
        (pp_table_bytes_vbios, "VBIOS"),
    ] {
        let Some(bytes) = bytes else {
            println!("    from {src}: N/A");
            continue;
        };

        let pp_table = if let Some(smu) = &smu {
            PPTable::decode_with_smu_version(&bytes, smu.version())
        } else {
            PPTable::decode(&bytes)
        };

        if let Ok (pp_table) = &pp_table {
            println!("    from {src}: {pp_table:#?}");
        } else {
            println!("    from {src}: N/A");
        }
    }

    println!();
}
