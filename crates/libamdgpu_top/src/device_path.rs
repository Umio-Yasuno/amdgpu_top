use anyhow::{anyhow, Context};
use libdrm_amdgpu_sys::{AMDGPU::DeviceHandle, PCI};
use std::path::PathBuf;
use std::fs;
use std::fmt;

const DRM_RENDER: u32 = 128;

#[derive(Clone)]
pub struct DevicePath {
    pub render: PathBuf,
    pub card: PathBuf,
    pub pci: Option<PCI::BUS_INFO>,
    pub instance_number: u32,
}

impl DevicePath {
    pub fn new(instance: u32) -> Self {
        let card = format!("/dev/dri/card{}", instance);
        let instance_number = card_to_instance_number(&card).unwrap();

        Self {
            render: PathBuf::from(format!("/dev/dri/renderD{}", DRM_RENDER + instance)),
            card: PathBuf::from(card),
            pci: None,
            instance_number,
        }
    }

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
            let pci = name.into_string().ok()?.parse::<PCI::BUS_INFO>().ok()?;

            Self::try_from(pci).ok()
        }).collect()
    }
}

fn card_to_instance_number(s: &str) -> Option<u32> {
    s.trim_start_matches("/dev/dri/card").parse::<u32>().ok()
}

impl TryFrom<PCI::BUS_INFO> for DevicePath {
    type Error = std::io::Error;

    fn try_from(pci: PCI::BUS_INFO) -> Result<Self, Self::Error> {
        let base = PathBuf::from("/dev/dri/by-path");

        let [render, card] = ["render", "card"].map(|v| -> std::io::Result<PathBuf> {
            let name = format!("pci-{pci}-{v}");
            let link = fs::read_link(base.join(name))?;

            fs::canonicalize(base.join(link))
        });

        let instance_number = card.as_ref().ok().and_then(|path|
            card_to_instance_number(path.to_str()?)
        ).unwrap_or_else(|| {
            eprintln!("invalid card path: {card:?}");
            panic!();
        });

        Ok(Self {
            render: render?,
            card: card?,
            pci: Some(pci),
            instance_number,
        })
    }
}

impl fmt::Debug for DevicePath {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("DevicePath")
            .field("render", &self.render)
            .field("card", &self.card)
            .field("pci", &self.pci.map(|pci| pci.to_string()))
            .finish()
    }
}
