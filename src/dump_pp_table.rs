use libamdgpu_top::{
    AMDGPU::{
        self,
        HwId,
        IpHwId,
        PPTable,
    },
    DevicePath,
};

pub fn dump_all_pp_table(title: &str, device_path_list: &[DevicePath]) {
    println!("{title}\n");

    for (i, device_path) in device_path_list.iter().enumerate() {
        println!("\n--------\n#{i}");
        dump_pp_table(device_path);
    }
}

fn dump_pp_table(device_path: &DevicePath) {
    let Ok(amdgpu_dev) = device_path.init() else { return };

    if let [Some(did), Some(rid)] = [device_path.device_id, device_path.revision_id] {
        println!(
            "{} ({}, {did:#0X}:{rid:#0X})",
            device_path.device_name,
            device_path.pci,
        );
    }

    let sysfs = &device_path.sysfs_path;
    let smu = IpHwId::get_from_die_id_sysfs(HwId::MP1, sysfs.join("ip_discovery/die/0/")).ok().and_then(|smu| smu.instances.first().cloned());

    if let Some(smu) = &smu {
        println!("    SMU (MP1) version: {}.{}.{}", smu.major, smu.minor, smu.revision);
    }

    let pp_table_bytes_sysfs = std::fs::read(sysfs.join("pp_table")).ok();
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
