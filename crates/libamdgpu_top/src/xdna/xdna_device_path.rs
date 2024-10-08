// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_drm.c

use std::fs;
use crate::DevicePath;
use crate::PCI;

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
const XDNA_DEVICE_IDS: &[u32] = &[
    0x1502, 
    0x17F0,
    0x1569,
    0x1640,
];

pub fn find_xdna_device() -> Option<DevicePath> {
    fs::read_dir(PCI_DEVICES_DIR).ok()?.find_map(|dir_entry| {
        let path = dir_entry.ok()?.path();

        {
            let vendor = fs::read_to_string(path.join("vendor")).ok()?;

            if vendor != "0x1022\n" {
                return None;
            }
        }

        {
            let device = fs::read_to_string(path.join("device")).ok()?;
            /* "0x1502\n", "0x17F0\n" */
            let device = u32::from_str_radix(device.get(2..device.len()-1)?, 16).ok()?;

            if XDNA_DEVICE_IDS.contains(&device) {
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
            (0x1502, 0x0) => "NPU1",
            (0x17F0, 0x0) => "NPU2",
            (0x1569, 0x0) |
            (0x1640, 0x0) => "NPU3",
            (0x17F0, 0x10) => "NPU4",
            (0x17F0, 0x11) => "NPU5",
            (0x17F0, 0x20) => "NPU6",
            _ => "",
        }.to_string();
    }
}
