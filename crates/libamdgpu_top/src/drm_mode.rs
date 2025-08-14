use std::fmt::Write;
use std::fs::File;
use crate::{
    LibDrm,
    drmModePropType,
    drmModeConnectorType,
    drmModeConnection,
    drmModeCrtc,
    drmModeModeInfo,
    drm_mode_property_enum,
};
use crate::DevicePath;

#[derive(Debug, Clone)]
pub struct ConnectorInfo {
    pub connector_id: u32,
    pub connector_type: drmModeConnectorType,
    pub connector_type_id: u32,
    pub connection: drmModeConnection,
    pub mode_info: Vec<drmModeModeInfo>,
    pub mode_props: Vec<(ModeProp, u64)>,
    pub crtc: Option<drmModeCrtc>,
}

impl ConnectorInfo {
    pub fn name(&self) -> String {
        format!(
            "Connector {} ({}-{}), {}",
            self.connector_id,
            self.connector_type,
            self.connector_type_id,
            self.connection,
        )
    }
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

impl ModeProp {
    pub fn enums_string(&self) -> String {
        let mut s = self.enums.iter().fold(String::new(), |mut s, enum_| {
            let _ = write!(s, "{:?}={}, ", enum_.name(), enum_.value);
            s
        });
        let len = s.len();
        let _ = s.split_off(len-2);

        s
    }
}

pub fn connector_info(device_path: &DevicePath) -> Vec<ConnectorInfo> {
    let Some(libdrm) = device_path.libdrm_amdgpu.clone().map(LibDrm::from) else {
        return Vec::new();
    };
    let fd = {
        use std::os::fd::IntoRawFd;

        let Some(f) = File::open(&device_path.card).ok() else { return Vec::new() };

        f.into_raw_fd()
    };

    libdrm.set_all_client_caps(fd);
    let Some(drm_mode_res) = libdrm.get_drm_mode_resources(fd) else { return Vec::new() };
    let current_connectors = drm_mode_res.get_drm_mode_all_connector_current(fd);

    let conn_info: Vec<ConnectorInfo> = current_connectors.iter().filter_map(|conn| {
        let connector_id = conn.connector_id();
        let connector_type = conn.connector_type();
        let connector_type_id = conn.connector_type_id();
        let connection = conn.connection();

        let conn_prop = conn.get_drm_mode_connector_properties(fd)?;
        let mode_info = conn.get_modes();
        let mode_props = conn_prop.get_mode_property(fd);

        let mode_props: Vec<(ModeProp, u64)> = mode_props.iter().map(|(prop, value)| {
            let prop_type = prop.property_type();
            let flags = prop.flags();
            let name = prop.name();
            let prop_id = prop.prop_id();
            let enums = prop.enums();
            let values = prop.values();

            let mode_prop = ModeProp {
                prop_type,
                prop_id,
                flags,
                name,
                enums,
                values,
            };

            (mode_prop, *value)
        }).collect();

        let crtc_id = mode_props
            .iter()
            .find(|prop| prop.0.name == "CRTC_ID")
            .map(|prop| prop.1);

        let crtc = if let Some(crtc_id) = crtc_id {
            drm_mode_res
                .get_drm_mode_all_crtcs(fd)
                .iter()
                .copied()
                .find(|crtc| crtc.crtc_id as u64 == crtc_id)
        } else {
            None
        };

        Some(ConnectorInfo {
            connector_id,
            connector_type,
            connector_type_id,
            connection,
            mode_info,
            mode_props,
            crtc,
        })
    }).collect();

    conn_info
}
