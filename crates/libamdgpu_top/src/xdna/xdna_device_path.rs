// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_drm.c
// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_pci_drv.c

use std::fs;
use std::path::Path;
use crate::{DevicePath, PCI};

/*
const DRIVER_NAME_1: &str = "/sys/bus/pci/drivers/amdxdna_accel_driver";

pub fn get_xdna_device_path() -> Option<DevicePath> {
    fs::read_dir(DRIVER_NAME_1).ok()?.find_map(|v| {
        let name = v.ok()?.file_name();

        /* 0000:00:00.0 */
        if name.len() < 12 { return None; }

        let pci: PCI::BUS_INFO = name.into_string().ok()?.parse().ok()?;

        DevicePath::try_from(pci).ok()
    }).map(|mut v| {
        v.fill_xdna_device_name();
        v
    })
}
*/

const PCI_DEVICES_DIR: &str = "/sys/bus/pci/devices";
const VENDOR_AMD: u32 = 0x1022;
const VENDOR_ATI: u32 = 0x1002;
const XDNA_NPU3_DEVICES: &[(u32, u32)] = &[
    /* (vendor, device) */
    (VENDOR_AMD, 0x1569),
    (VENDOR_ATI, 0x1640),
];

fn parse_sysfs_hex<P: AsRef<Path>>(path: P) -> Option<u32> {
    let s = fs::read_to_string(path.as_ref()).ok()?;

    u32::from_str_radix(s.get(2..s.len()-1)?, 16).ok()
}

fn is_amd_signal_processing(vendor: u32, class: u32) -> bool {
    vendor == 0x1022 && class == 0x118000
}

pub fn find_xdna_device() -> Option<DevicePath> {
    fs::read_dir(PCI_DEVICES_DIR).ok()?.find_map(|dir_entry| {
        let path = dir_entry.ok()?.path();

        {
            let vendor = parse_sysfs_hex(path.join("vendor"))?;

            if !&[VENDOR_AMD, VENDOR_ATI].contains(&vendor) {
                return None;
            }

            let device = parse_sysfs_hex(path.join("device"))?;
            let class = parse_sysfs_hex(path.join("class"))?;

            if !XDNA_NPU3_DEVICES.contains(&(vendor, device)) && !is_amd_signal_processing(vendor, class) {
                return None;
            }
        }

        let pci: PCI::BUS_INFO = path.file_name()?.to_str()?.parse().ok()?;

        DevicePath::try_from(pci).ok()
    }).map(|mut v| {
        v.fill_xdna_device_name();
        v
    })
}

impl DevicePath {
    pub fn fill_xdna_device_name(&mut self) {
        let [Some(device_id), Some(revision_id)] = [self.device_id, self.revision_id] else {
            return;
        };

        // ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_pci_drv.c
        self.device_name = match (device_id, revision_id) {
            (0x1502, 0x0) => "NPU1".to_string(),
            (0x17F0, 0x0) => "NPU2".to_string(),
            (0x1569, 0x0) |
            (0x1640, 0x0) => "NPU3".to_string(),
            (0x17F0, 0x10) => "NPU4".to_string(),
            (0x17F0, 0x11) => "NPU5".to_string(),
            (0x17F0, 0x20) => "NPU6".to_string(),
            _ => format!("NPU ({device_id:#06X}:{revision_id:#04X})"),
        };
    }
}
