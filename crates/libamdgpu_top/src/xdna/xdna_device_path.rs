// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_drm.c
// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_pci_drv.c

use std::{fs, io};
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
    // 0x11: Signal Processing Controller, 0x80: Other
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
        // ref: https://github.com/amd/xdna-driver/blob/main/src/driver/doc/sysfs-driver-amd-aie
        self.device_name = std::fs::read_to_string(self.sysfs_path.join("vbnv"))
            .unwrap_or(format!("RyzenAI-npu ({device_id:#06X}:{revision_id:#04X})"));
    }

    pub fn get_xdna_fw_version(&self) -> io::Result<String> {
        std::fs::read_to_string(self.sysfs_path.join("fw_version"))
            .map(|mut s| {
                let _ = s.pop(); // trim '\n'
                s
            })
    }
}
