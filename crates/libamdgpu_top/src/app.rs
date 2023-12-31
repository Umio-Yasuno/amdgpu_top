use crate::AMDGPU::{ASIC_NAME, DeviceHandle, GPU_INFO, GpuMetrics};
use crate::{DevicePath, stat, VramUsage, has_vcn, has_vcn_unified, Sampling};
use stat::{FdInfoStat, GpuActivity, Sensors, PcieBw, PerfCounter, ProcInfo};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use crate::AppDeviceInfo;

pub struct AppAmdgpuTop {
    pub amdgpu_dev: DeviceHandle,
    pub device_info: AppDeviceInfo,
    pub device_path: DevicePath,
    pub stat: AppAmdgpuTopStat,
}

#[derive(Clone)]
pub struct AppAmdgpuTopStat {
    pub grbm: PerfCounter,
    pub grbm2: PerfCounter,
    pub vram_usage: VramUsage,
    pub sensors: Sensors,
    pub metrics: Option<GpuMetrics>,
    pub activity: GpuActivity,
    pub fdinfo: FdInfoStat,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub arc_pcie_bw: Option<Arc<Mutex<PcieBw>>>,
}

pub struct AppOption {
    pub pcie_bw: bool,
}

impl Default for AppOption {
    fn default() -> Self {
        Self {
            pcie_bw: false,
        }
    }
}

impl AppAmdgpuTop {
    pub fn from_device_path_list<T: AsRef<AppOption>>(
        device_path_list: &[DevicePath],
        opt: T,
    ) -> Vec<Self> {
        let vec_json_device: Vec<Self> = device_path_list.iter().filter_map(|device_path| {
            let amdgpu_dev = device_path.init().ok()?;

            Self::new(amdgpu_dev, device_path.clone(), opt.as_ref())
        }).collect();

        vec_json_device
    }

    pub fn new(amdgpu_dev: DeviceHandle, device_path: DevicePath, opt: &AppOption) -> Option<Self> {
        let pci_bus = device_path.pci;
        let ext_info = amdgpu_dev.device_info().ok()?;
        let asic_name = ext_info.get_asic_name();
        let memory_info = amdgpu_dev.memory_info().ok()?;
        let sysfs_path = pci_bus.get_sysfs_path();

        let [grbm, grbm2] = {
            let chip_class = ext_info.get_chip_class();

            [
                PerfCounter::new_with_chip_class(stat::PCType::GRBM, chip_class),
                PerfCounter::new_with_chip_class(stat::PCType::GRBM2, chip_class),
            ]
        };

        let vram_usage = VramUsage::new(&memory_info);
        let sensors = Sensors::new(&amdgpu_dev, &pci_bus, &ext_info);

        let metrics = amdgpu_dev.get_gpu_metrics_from_sysfs_path(&sysfs_path).ok();
        let activity = GpuActivity::get(&amdgpu_dev, &sysfs_path, asic_name);

        let arc_proc_index = {
            let mut proc_index: Vec<ProcInfo> = Vec::new();
            stat::update_index(&mut proc_index, &device_path);

            Arc::new(Mutex::new(proc_index))
        };

        let arc_pcie_bw = if opt.pcie_bw {
            let pcie_bw = PcieBw::new(pci_bus.get_sysfs_path());

            if pcie_bw.check_pcie_bw_support(&ext_info) {
                Some(pcie_bw.spawn_update_thread())
            } else {
                None
            }
        } else {
            None
        };

        let fdinfo = FdInfoStat {
            has_vcn: has_vcn(&amdgpu_dev),
            has_vcn_unified: has_vcn_unified(&amdgpu_dev),
            ..Default::default()
        };

        let mut device_info = AppDeviceInfo::new(
            &amdgpu_dev,
            &ext_info,
            &memory_info,
            &sensors,
        );

        device_info.gfx_target_version = device_path.get_gfx_target_version_from_kfd();

        Some(Self {
            amdgpu_dev,
            device_info,
            device_path,
            stat: AppAmdgpuTopStat {
                grbm,
                grbm2,
                vram_usage,
                sensors,
                metrics,
                activity,
                fdinfo,
                arc_proc_index,
                arc_pcie_bw,
            },
        })
    }

    pub fn update(&mut self, interval: Duration) {
        self.stat.vram_usage.update_usage(&self.amdgpu_dev);
        self.stat.sensors.update(&self.amdgpu_dev);
        self.stat.metrics = GpuMetrics::get_from_sysfs_path(&self.device_info.sysfs_path).ok();

        self.stat.activity = if let Some(metrics) = &self.stat.metrics {
            GpuActivity::from_gpu_metrics(metrics)
        } else {
            // Some Raven/Picasso/Raven2 APU always report gpu_busy_percent as 100.
            // ref: https://gitlab.freedesktop.org/drm/amd/-/issues/1932
            // gpu_metrics is supported from Renoir APU.
            match self.device_info.asic_name {
                ASIC_NAME::CHIP_RAVEN |
                ASIC_NAME::CHIP_RAVEN2 => GpuActivity { gfx: None, umc: None, media: None },
                _ => GpuActivity::get_from_sysfs(&self.device_info.sysfs_path),
            }
        };

        {
            let lock = self.stat.arc_proc_index.try_lock();
            if let Ok(proc_index) = lock {
                self.stat.fdinfo.get_all_proc_usage(&proc_index);
                self.stat.fdinfo.interval = interval;
            } else {
                self.stat.fdinfo.interval += interval;
            }
        }

        if self.stat.activity.media.is_none() || self.stat.activity.media == Some(0) {
            self.stat.activity.media = self.stat.fdinfo.fold_fdinfo_usage().media.try_into().ok();
        }
    }

    pub fn update_pc(&mut self) {
        self.stat.grbm.read_reg(&self.amdgpu_dev);
        self.stat.grbm2.read_reg(&self.amdgpu_dev);
    }

    pub fn update_pc_with_sampling(&mut self, sample: &Sampling) {
        self.clear_pc();

        for _ in 0..sample.count {
            self.update_pc();
            std::thread::sleep(sample.delay);
        }
    }

    pub fn clear_pc(&mut self) {
        self.stat.grbm.bits.clear();
        self.stat.grbm2.bits.clear();
    }
}
