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
    let len = vec_conn_info.len() - 1;

    println!("\nNode: {:?}", device_path.card);

    for (i, conn) in vec_conn_info.iter().enumerate() {
        let last = i == len;

        println!(
            "{}───{}",
            if last { "└" } else { "├" },
            conn.name(),
        );

        let props_len = conn.mode_props.len() - 1;

        for (j, mode_prop) in conn.mode_props.iter().enumerate() {
            let last_prop = j == props_len;
            dump_mode_prop(mode_prop, last, last_prop);
        }
    }
}

pub fn dump_mode_prop((mode_prop, value): &(ModeProp, u64), last: bool, last_prop: bool) {
    println!(
        "{}    {}───{:?}, id = {}, value: {}{}",
        if last { " " } else { "│" },
        if last_prop { "└" } else { "├" },
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
