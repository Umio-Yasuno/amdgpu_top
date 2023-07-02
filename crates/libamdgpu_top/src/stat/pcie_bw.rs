use crate::AMDGPU::{drm_amdgpu_info_device, GPU_INFO, ASIC_NAME};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    pub sent: Option<u64>,
    pub received: Option<u64>,
    pub max_payload_size: Option<i32>,
}

impl PcieBw {
    pub fn new<P: Into<PathBuf>>(sysfs_path: P) -> Self {
        let path = sysfs_path.into().join("pcie_bw");

        Self {
            path,
            sent: None,
            received: None,
            max_payload_size: None,
        }
    }

    pub fn update(&mut self) {
        let Ok(s) = fs::read_to_string(&self.path) else { return };
        let mut split = s.trim_end().split(' ');

        self.sent = split.next().and_then(|v| v.parse().ok());
        self.received = split.next().and_then(|v| v.parse().ok());
        self.max_payload_size = split.next().and_then(|v| v.parse().ok());
    }

    pub fn spawn_update_thread(&self) -> Arc<Mutex<Self>> {
        let arc = Arc::new(Mutex::new(self.clone()));
        let arc_pcie_bw = arc.clone();
        let mut buf_pcie_bw = self.clone();

        std::thread::spawn(move || loop {
            buf_pcie_bw.update(); // msleep(1000)

            if buf_pcie_bw.sent.is_none()
            && buf_pcie_bw.received.is_none()
            && buf_pcie_bw.max_payload_size.is_none() {
                return;
            }

            let lock = arc.lock();
            if let Ok(mut pcie_bw) = lock {
                *pcie_bw = buf_pcie_bw.clone();
            }

            std::thread::sleep(Duration::from_millis(500)); // wait for user input
        });

        arc_pcie_bw
    }

    pub fn check_pcie_bw_support(&self, ext_info: &drm_amdgpu_info_device) -> bool {
        // APU and RDNA GPU dose not support `pcie_bw`.
        // ref: https://lists.freedesktop.org/archives/amd-gfx/2020-May/049649.html
        self.path.exists()
        && !ext_info.is_apu()
        && ext_info.get_asic_name() < ASIC_NAME::CHIP_NAVI10
    }
}
