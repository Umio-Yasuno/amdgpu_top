use crate::{
    drmModeRes,
    drmModePropType,
    drmModeConnectorType,
    drmModeConnection,
    drm_mode_property_enum,
};
use std::fs::File;
use crate::DevicePath;

#[derive(Debug, Clone)]
pub struct ConnectorInfo {
    pub connector_id: u32,
    pub connector_type: drmModeConnectorType,
    pub connector_type_id: u32,
    pub connection: drmModeConnection,
    pub mode_props: Vec<(ModeProp, u64)>
}

#[derive(Debug, Clone)]
pub struct ModeProp {
    pub prop_type: drmModePropType,
    pub prop_id: u32,
    pub flags: u32,
    pub name: String,
    pub values: Vec<u64>,
    pub enums: Vec<drm_mode_property_enum>,
}

pub fn connector_info(device_path: &DevicePath) -> Vec<ConnectorInfo> {
    let fd = {
        use std::os::fd::IntoRawFd;

        let Some(f) = File::open(&device_path.card).ok() else { return Vec::new() };

        f.into_raw_fd()
    };

    let Some(drm_mode_res) = drmModeRes::get(fd) else { return Vec::new() };
    let current_connectors = drm_mode_res.get_all_connector_current(fd);

    let conn_info: Vec<ConnectorInfo> = current_connectors.iter().filter_map(|conn| {
        let connector_id = conn.connector_id();
        let connector_type = conn.connector_type();
        let connector_type_id = conn.connector_type_id();
        let connection = conn.connection();

        let conn_prop = conn.get_connector_props(fd)?;
        let mode_props = conn_prop.get_mode_property(fd);

        let mode_props: Vec<(ModeProp, u64)> = mode_props.iter().map(|(prop, value)| {
            let prop_type = prop.property_type();
            let flags = prop.flags();
            let name = prop.name();
            let prop_id = prop.prop_id();

            let mode_prop = ModeProp {
                prop_type,
                prop_id,
                flags,
                name,
                enums: prop.enums(),
                values: prop.values(),
            };

            (mode_prop, *value)
        }).collect();

        Some(ConnectorInfo {
            connector_id,
            connector_type,
            connector_type_id,
            connection,
            mode_props,
        })
    }).collect();

    conn_info
}
