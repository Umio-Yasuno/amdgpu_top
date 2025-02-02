// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_drm.c
// ref: https://github.com/amd/xdna-driver/blob/main/src/driver/amdxdna/amdxdna_pci_drv.c

use std::{fs, io};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use crate::{DeviceType, DevicePath, PCI};

pub fn find_xdna_device() -> Option<DevicePath> {
    let [accel, sysfs_path] = find_accel_path_and_sysfs_path()?;
    let pci: PCI::BUS_INFO = sysfs_path.file_name()?.to_str()?.parse().ok()?;
    let render = PathBuf::new();
    let card = PathBuf::new();
    let [device_id, revision_id] = [pci.get_device_id(), pci.get_revision_id()];
    let device_name = fs::read_to_string(sysfs_path.join("vbnv"))
        .map(|mut s| {
            let _ = s.pop(); // trim '\n'
            s
        })
        .unwrap_or_default();
    let arc_proc_index = Arc::new(Mutex::new(Vec::new()));
    let config_pm = sysfs_path.join("power").exists();

    Some(DevicePath {
        libdrm_amdgpu: None,
        render,
        card,
        accel,
        pci,
        sysfs_path,
        device_id,
        revision_id,
        device_name,
        arc_proc_index,
        config_pm,
        device_type: DeviceType::AMDXDNA,
    })
}

fn find_accel_path_and_sysfs_path() -> Option<[PathBuf; 2]> {
    const ACCEL_MAJOR: usize = 261;
    const MAX_MINOR: usize = 64;

    for i in 0..MAX_MINOR {
        let accel_path = PathBuf::from(format!("/dev/accel/accel{i}"));
        // let sysfs_path = PathBuf::from(format!("/sys/class/accel/accel{i}"));

        if !accel_path.exists() {
            continue;
        }

        // ref: https://github.com/intel/linux-npu-driver/blob/93fb54b9d42e7f0f6590f9134aaac92bbf226909/umd/vpu_driver/source/os_interface/vpu_driver_api.cpp#L357
        let sysfs_path = PathBuf::from(format!("/sys/dev/char/{ACCEL_MAJOR}:{i}/device/"));
        let sysfs_path = fs::canonicalize(sysfs_path).ok()?;
        let device_type_path = sysfs_path.join("device_type");
        let vbnv_path = sysfs_path.join("vbnv");

        // ref: https://github.com/amd/xdna-driver/blob/main/src/shim/pcidrv.cpp
        if device_type_path.exists() && vbnv_path.exists() {
            return Some([accel_path, sysfs_path]);
        }
    }

    None
}

impl DevicePath {
    pub fn get_xdna_fw_version(&self) -> io::Result<String> {
        std::fs::read_to_string(self.sysfs_path.join("fw_version"))
            .map(|mut s| {
                let _ = s.pop(); // trim '\n'
                s
            })
    }
}
