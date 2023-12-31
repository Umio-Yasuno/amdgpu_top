use anyhow::{anyhow, Context};
use libdrm_amdgpu_sys::{AMDGPU::DeviceHandle, PCI};
use std::path::PathBuf;
use std::fs;
use std::fmt;
use crate::GfxTargetVersion;

// const DRM_RENDER: u32 = 128;

#[derive(Clone)]
pub struct DevicePath {
    pub render: PathBuf,
    pub card: PathBuf,
    pub pci: PCI::BUS_INFO,
}

impl DevicePath {
    pub fn init(&self) -> anyhow::Result<DeviceHandle> {
        let (amdgpu_dev, _major, _minor) = {
            use std::os::unix::io::IntoRawFd;

            // need write option for GUI context
            // https://gitlab.freedesktop.org/mesa/mesa/-/issues/2424
            let f = fs::OpenOptions::new().read(true).write(true).open(&self.render)?;

            DeviceHandle::init(f.into_raw_fd())
                .map_err(|v| anyhow!(v))
                .context("Failed to DeviceHandle::init")?
        };

        Ok(amdgpu_dev)
    }

    pub fn get_device_path_list() -> Vec<Self> {
        let amdgpu_devices = fs::read_dir("/sys/bus/pci/drivers/amdgpu").unwrap_or_else(|_| {
            eprintln!("The AMDGPU driver is not loaded.");
            panic!();
        });

        amdgpu_devices.flat_map(|v| {
            let name = v.ok()?.file_name();

            /* 0000:00:00.0 */
            if name.len() < 12 { return None; }

            let pci = name.into_string().ok()?.parse::<PCI::BUS_INFO>().ok()?;

            Self::try_from(pci).ok()
        }).collect()
    }

    pub fn get_gfx_target_version_from_kfd(&self) -> Option<GfxTargetVersion> {
        let drm_render_minor = {
            const PRE: &str = "/dev/dri/renderD";
            const PRE_LEN: usize = PRE.len();
            let render = self.render.to_str()?;
            if !render.starts_with(PRE) { return None }

            format!("drm_render_minor {}", &render[PRE_LEN..])
        };

        let dirs = std::fs::read_dir("/sys/class/kfd/kfd/topology/nodes/").ok()?;
        let mut gfx_target_version = String::with_capacity(32);

        'node: for dir_entry in dirs.flatten() {
            gfx_target_version.clear();
            let Ok(s) = std::fs::read_to_string(dir_entry.path().join("properties")) else {
                continue
            };
            let lines = s.lines();

            for l in lines {
                if l.starts_with("gfx_target_version") {
                    gfx_target_version = l.to_string();
                }

                if l.starts_with(&drm_render_minor) {
                    break 'node;
                }
            }
        }

        const PRE_GFX_VER_LEN: usize = "gfx_target_version ".len();
        let gfx_target_version: u32 = gfx_target_version[PRE_GFX_VER_LEN..].parse().ok()?;

        Some(GfxTargetVersion::from(gfx_target_version))
    }
}

impl TryFrom<PCI::BUS_INFO> for DevicePath {
    type Error = std::io::Error;

    fn try_from(pci: PCI::BUS_INFO) -> Result<Self, Self::Error> {
        let render = pci.get_drm_render_path()?;
        let card = pci.get_drm_card_path()?;

        Ok(Self { render, card, pci })
    }
}

impl fmt::Debug for DevicePath {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("DevicePath")
            .field("render", &self.render)
            .field("card", &self.card)
            .field("pci", &self.pci.to_string())
            .finish()
    }
}
