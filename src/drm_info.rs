use libamdgpu_top::{
    DevicePath,
    drmModePropType,
    // ConnectorInfo,
    ModeProp,
};

pub fn dump_all_drm_info(device_path_list: &[DevicePath]) {
    for device_path in device_path_list {
        dump_drm_info(device_path);
        println!();
    }
}

pub fn dump_drm_info(device_path: &DevicePath) {
    let vec_conn_info = libamdgpu_top::connector_info(device_path);

    println!("\nNode: {:?}", device_path.card);

    for conn in vec_conn_info {
        println!("├───{}", conn.name());

        for mode_prop in &conn.mode_props {
            dump_mode_prop(mode_prop);
        }
    }
}

pub fn dump_mode_prop((mode_prop, value): &(ModeProp, u64)) {
    println!(
        "├───────{:?}, id = {}, value: {}{}",
        mode_prop.name,
        mode_prop.prop_id,
        value,
        match mode_prop.prop_type {
            drmModePropType::BLOB => format!(", blob"),
            drmModePropType::RANGE => format!(", values: {:?}", mode_prop.values),
            drmModePropType::ENUM => {
                let enums: String = mode_prop.enums.iter().map(|enum_| {
                    format!("{:?}={}, ", enum_.name(), enum_.value)
                }).collect();
                let len = enums.len();

                format!(", enums: [{}]", &enums[..len-2])
            },
            _ => "".to_string(),
        },
    );
}
