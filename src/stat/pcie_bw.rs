use std::fs;
use std::path::PathBuf;

// PCIe bandwidth (throughput) available from `pcie_bw` sysfs
// `pcie_bw` is supported on dGPUs only
// The AMDGPU driver waits 1s (`msleep(1000)`) for pcie performance counters.
// So we should read the file in a separate thread.

// ## Reference
//  * <https://www.kernel.org/doc/html/latest/gpu/amdgpu/driver-misc.html#pcie-bw>
//  * <https://github.com/RadeonOpenCompute/rocm_smi_lib>
//  * Linux Kernel
//    * `drivers/gpu/drm/amd/amdgpu_pm.c`
//    * `drivers/gpu/drm/amd/amdgpu/{cik,si,vi,soc15}.c`
//      * `{cik,si,vi,soc15}_get_pcie_usage`

#[derive(Clone, Debug)]
pub struct PcieBw {
    path: PathBuf,
    pub exists: bool,
    pub sent: u64,
    pub received: u64,
    pub max_payload_size: i32,
}

impl PcieBw {
    pub fn new<P: Into<PathBuf>>(sysfs_path: P) -> Self {
        let path = sysfs_path.into().join("pcie_bw");
        let exists = path.exists();

        Self {
            path,
            exists,
            sent: 0,
            received: 0,
            max_payload_size: 0,
        }
    }

    pub fn update(&mut self) {
        let Ok(s) = fs::read_to_string(&self.path) else { return };
        let split: Vec<&str> = s.trim_end().split(' ').collect();

        if let Some(sent) = split.get(0).and_then(|v| v.parse().ok()) {
            self.sent = sent;
        }

        if let Some(rec) = split.get(1).and_then(|v| v.parse().ok()) {
            self.received = rec;
        }

        if let Some(mps) = split.get(2).and_then(|v| v.parse().ok()) {
            self.max_payload_size = mps;
        }
    }
}
