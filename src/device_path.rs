use crate::MainOpt;
use anyhow::{anyhow, Context};
use libdrm_amdgpu_sys::AMDGPU::DeviceHandle;
use libdrm_amdgpu_sys::PCI;
use std::path::PathBuf;
use std::fs;
use std::fmt;

#[derive(Clone)]
pub struct DevicePath {
    pub render: PathBuf,
    pub card: PathBuf,
    pub pci: Option<PCI::BUS_INFO>,
}

impl DevicePath {
    fn new(instance: u32) -> Self {
        Self {
            render: PathBuf::from(format!("/dev/dri/renderD{}", 128 + instance)),
            card: PathBuf::from(format!("/dev/dri/card{}", instance)),
            pci: None,
        }
    }

    fn from_pci(pci_path: &str) -> anyhow::Result<Self> {
        let base = PathBuf::from("/dev/dri/by-path");

        let [render, card] = ["render", "card"].map(|v| {
            let name = format!("pci-{pci_path}-{v}");
            let link = fs::read_link(base.join(&name)).map_err(|err| {
                eprintln!("Error: {err}");
                eprintln!("path: {name}");

                anyhow!(format!("pci: {pci_path}"))
            })?;

            fs::canonicalize(base.join(link)).map_err(|err| anyhow!(err))
        });

        let pci = PCI::BUS_INFO::from_number_str(pci_path); // pci_path.parse().ok()

        Ok(Self {
            render: render?,
            card: card?,
            pci,
        })
    }

    pub fn from_main_opt(main_opt: &MainOpt, list: &[Self]) -> (Self, DeviceHandle) {
        // default
        if main_opt.instance == 0 && main_opt.pci_path.is_none() {
            return Self::init_with_fallback(main_opt, list);
        }

        let device_path = if let Some(ref pci_path) = main_opt.pci_path {
            Self::from_pci(pci_path).unwrap_or_else(|err| {
                eprintln!("{err}");
                eprintln!("Device list: {list:#?}");
                panic!();
            })
        } else {
            Self::new(main_opt.instance)
        };
        let amdgpu_dev = device_path.init().unwrap_or_else(|err| {
            eprintln!("{err}");
            eprintln!("{:?}", device_path);
            eprintln!("Device list: {list:#?}");
            panic!();
        });

        (device_path, amdgpu_dev)
    }

    pub fn init(&self) -> anyhow::Result<DeviceHandle> {
        let (amdgpu_dev, _major, _minor) = {
            use std::os::fd::IntoRawFd;

            // need write option for GUI context
            // https://gitlab.freedesktop.org/mesa/mesa/-/issues/2424
            let f = fs::OpenOptions::new().read(true).write(true).open(&self.render)?;

            DeviceHandle::init(f.into_raw_fd()).map_err(|v| anyhow!(v))
                .context("Failed DeviceHandle::init")?
        };

        Ok(amdgpu_dev)
    }

    fn fallback(main_opt: &MainOpt) -> anyhow::Result<(Self, DeviceHandle)> {
        if let Some(ref pci_path) = main_opt.pci_path {
            let device_path = Self::from_pci(pci_path)?;
            let amdgpu_dev = device_path.init()?;

            return Ok((device_path, amdgpu_dev));
        }

        let device_path = Self::new(main_opt.instance);
        let amdgpu_dev = match device_path.init() {
            Ok(amdgpu_dev) => amdgpu_dev,
            Err(err) => {
                eprintln!("{err}");
                return Err(err).with_context(|| format!("Error: {device_path:?}"));
            },
        };

        Ok((device_path, amdgpu_dev))
    }

    fn init_with_fallback(main_opt: &MainOpt, list: &[Self]) -> (Self, DeviceHandle) {
        Self::fallback(main_opt).unwrap_or_else(|err| {
            eprintln!("{err}");
            eprintln!("Fallback: list: {list:#?}");
            let device_path = DevicePath::fallback_device_path(list);
            let amdgpu_dev = device_path.init().unwrap();
            eprintln!("Fallback: to: {device_path:?}");

            (device_path, amdgpu_dev)
        })
    }

    pub fn get_instance_number(&self) -> Option<u32> {
        let card = self.card.to_str()?;

        card.trim_start_matches("/dev/dri/card").parse::<u32>().ok()
    }

    pub fn get_device_path_list() -> Vec<Self> {
        let mut dev_paths = Vec::new();

        const PRE: usize = "pci-".len();
        const PCI: usize = "0000:00:00.0".len();
        const SYS_BUS: &str = "/sys/bus/pci/devices/";

        let by_path = fs::read_dir("/dev/dri/by-path").unwrap();

        for path in by_path.flatten() {
            // "pci-0000:06:00.0-render"
            let Ok(path) = path.file_name().into_string() else { continue };
            if !path.ends_with("render") { continue }

            let pci = {
                if path.len() < PRE+PCI { continue }
                &path[PRE..PRE+PCI]
            };

            let Ok(uevent) = fs::read_to_string(
                PathBuf::from(SYS_BUS).join(pci).join("uevent")
            ) else { continue };

            if uevent.lines().any(|line| line.starts_with("DRIVER=amdgpu")) {
                if let Ok(path) = DevicePath::from_pci(pci) {
                    dev_paths.push(path);
                }
            }
        }

        if dev_paths.is_empty() { panic!("AMD GPU not found.") };

        dev_paths
    }

    pub fn fallback_device_path(list: &[Self]) -> Self {
        list.get(0).unwrap_or_else(|| panic!("AMD GPU not found.")).clone()
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
