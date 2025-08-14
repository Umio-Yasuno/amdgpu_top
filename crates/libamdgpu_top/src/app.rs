use crate::drmVersion;
use crate::AMDGPU::{DeviceHandle, GPU_INFO, GpuMetrics, MetricsInfo, RasBlock, RasErrorCount};
use crate::{AppDeviceInfo, DevicePath, stat, xdna, VramUsage, has_vcn, has_vcn_unified, has_vpe};
use stat::{FdInfoStat, GpuActivity, Sensors, PcieBw, PerfCounter, ProcInfo};
use xdna::XdnaFdInfoStat;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct AppAmdgpuTop {
    amdgpu_dev: ManuallyDrop<Option<DeviceHandle>>,
    pub device_info: AppDeviceInfo,
    pub device_path: DevicePath,
    pub xdna_device_path: Option<DevicePath>,
    pub xdna_fw_version: Option<String>,
    pub stat: AppAmdgpuTopStat,
    buf_interval: Duration,
    no_drop_device_handle: bool,
    dynamic_no_pc: bool, // to transition the APU into GFXOFF state
}

#[derive(Clone)]
pub struct AppAmdgpuTopStat {
    pub grbm: PerfCounter,
    pub grbm2: PerfCounter,
    pub vram_usage: VramUsage,
    pub sensors: Option<Sensors>,
    pub metrics: Option<GpuMetrics>,
    pub activity: GpuActivity,
    pub fdinfo: FdInfoStat,
    pub xdna_fdinfo: XdnaFdInfoStat,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub arc_xdna_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub arc_pcie_bw: Option<Arc<Mutex<PcieBw>>>,
    pub memory_error_count: Option<RasErrorCount>,
}

impl AppAmdgpuTopStat {
    // Workaround:
    // Raphael/Granite Ridge APU (SMU v13.0.5) reports very low input power. (7.00 - 15.00 mW)
    //     ref: https://gitlab.freedesktop.org/drm/amd/-/issues/2321
    //     ref: https://gitlab.freedesktop.org/drm/amd/-/issues/3999
    fn workaround_for_smu_v13_0_5(&mut self) -> Option<()> {
        let input_power = self.sensors.as_mut()?.input_power.as_mut()?;

        if input_power.value == 0 {
            let avg_socket_power = self.metrics.as_ref()?.get_average_socket_power()?;
            input_power.value = avg_socket_power;
        }

        None
    }

    // Workaround:
    // `*_get_fan_speed_rpm` is missing in `drivers/gpu/drm/amd/pm/swsmu/smu14/smu_v14_0_2_ppt.c`,
    // so SMU v14.0.2/3 dose not have `fan1_input` in `hwmon`.
    //     ref: https://gitlab.freedesktop.org/drm/amd/-/issues/4034
    //     ref: https://github.com/Umio-Yasuno/amdgpu_top/issues/123
    fn workaround_fan_rpm_for_smu_v14(&mut self) -> Option<()> {
        let sensors = self.sensors.as_mut()?;
        if sensors.fan_rpm.is_none() {
            sensors.fan_rpm = self.metrics
                .as_ref()?
                .get_current_fan_speed()
                .map(|fan_rpm| fan_rpm as u32);
        }

        None
    }
}

pub struct AppOption {
    pub pcie_bw: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AppOption {
    fn default() -> Self {
        Self {
            pcie_bw: false,
        }
    }
}

impl AppAmdgpuTop {
    pub fn create_app_and_suspended_list(
        device_path_list: &[DevicePath],
        opt: &AppOption,
    ) -> (Vec<Self>, Vec<DevicePath>) {
        let mut apps = Vec::new();
        let mut suspended_devices = Vec::new();

        for device_path in device_path_list {
            if !device_path.check_if_device_is_active() {
                suspended_devices.push(device_path.clone());
                continue;
            }

            let Ok(amdgpu_dev) = device_path.init() else { continue };
            let Some(app) = Self::new(amdgpu_dev, device_path.clone(), opt) else {
                continue
            };
            apps.push(app);
        }

        if apps.is_empty() && !suspended_devices.is_empty() {
            let (device_path, other_sus_devs) = suspended_devices.split_first().unwrap();
            // wake up
            let amdgpu_dev = device_path.init().unwrap();
            let app = AppAmdgpuTop::new(
                amdgpu_dev,
                device_path.clone(),
                &Default::default(),
            ).unwrap();
            apps.push(app);
            suspended_devices = other_sus_devs.to_vec();
        }

        (apps, suspended_devices)
    }

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
        let sysfs_path = device_path.sysfs_path.clone();
        let ext_info = amdgpu_dev.device_info().ok()?;
        let asic_name = ext_info.get_asic_name();
        let memory_info = amdgpu_dev.memory_info().ok()?;
        let no_drop_device_handle = if let Ok(s) = std::env::var("AGT_NO_DROP") {
            s == "1"
        } else {
            false
        };

        let [grbm, grbm2] = {
            let chip_class = ext_info.get_chip_class();

            [
                PerfCounter::new_with_chip_class(stat::PCType::GRBM, chip_class),
                PerfCounter::new_with_chip_class(stat::PCType::GRBM2, chip_class),
            ]
        };

        let vram_usage = VramUsage::new(&memory_info);
        let memory_error_count = RasErrorCount::get_from_sysfs_with_ras_block(&sysfs_path, RasBlock::UMC).ok();

        let sensors = Sensors::new(&amdgpu_dev, &pci_bus, &ext_info);
        let metrics = GpuMetrics::get_from_sysfs_path(&device_path.sysfs_path).ok();
        let activity = GpuActivity::get(&sysfs_path, asic_name);

        let arc_pcie_bw = if opt.pcie_bw {
            let pcie_bw = PcieBw::new(&sysfs_path);

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
            has_vpe: has_vpe(&amdgpu_dev),
            ..Default::default()
        };
        let xdna_fdinfo = XdnaFdInfoStat::default();

        let mut device_info = AppDeviceInfo::new(
            &amdgpu_dev,
            &ext_info,
            &memory_info,
            &sensors,
            &device_path,
        );

        if device_info.gfx_target_version.is_none() {
            device_info.gfx_target_version =
                device_path.get_gfx_target_version_from_kfd().map(|v| v.to_string());
        }

        let xdna_device_path = if device_info.has_npu {
            xdna::find_xdna_device()
        } else {
            None
        };
        let xdna_fw_version = xdna_device_path.as_ref().and_then(|d| d.get_xdna_fw_version().ok());

        let arc_proc_index = device_path.arc_proc_index.clone();
        let arc_xdna_proc_index = xdna_device_path
            .as_ref()
            .map(|v| v.arc_proc_index.clone())
            .unwrap_or_default();

        {
            let mut proc_index = arc_proc_index.lock().unwrap();
            let all_procs = stat::get_process_list();

            stat::update_index_by_all_proc(
                &mut proc_index,
                &[&device_path.render, &device_path.card],
                &all_procs,
            );

            if let Some(xdna) = xdna_device_path.as_ref() {
                let mut xdna_proc_index = xdna.arc_proc_index.lock().unwrap();

                stat::update_index_by_all_proc(
                    &mut xdna_proc_index,
                    &[&xdna.accel],
                    &all_procs,
                );
            }
        }

        Some(Self {
            amdgpu_dev: ManuallyDrop::new(Some(amdgpu_dev)),
            device_info,
            device_path,
            xdna_device_path,
            xdna_fw_version,
            stat: AppAmdgpuTopStat {
                grbm,
                grbm2,
                vram_usage,
                sensors,
                metrics,
                activity,
                fdinfo,
                xdna_fdinfo,
                arc_proc_index,
                arc_xdna_proc_index,
                arc_pcie_bw,
                memory_error_count,
            },
            buf_interval: Duration::ZERO,
            no_drop_device_handle,
            dynamic_no_pc: false,
        })
    }

    pub fn update(&mut self, interval: Duration) {
        if interval != Duration::ZERO {
            let fdinfo_lock = self.stat.arc_proc_index.try_lock();
            let xdna_fdinfo_lock = self.stat.arc_xdna_proc_index.try_lock();

            if let [Ok(proc_index), Ok(xdna_proc_index)] = [fdinfo_lock, xdna_fdinfo_lock] {
                let fdinfo_interval = interval + self.buf_interval;
                self.stat.fdinfo.interval = fdinfo_interval;
                self.stat.xdna_fdinfo.interval = fdinfo_interval;

                self.stat.fdinfo.get_all_proc_usage(&proc_index);
                self.stat.xdna_fdinfo.get_all_proc_usage(&xdna_proc_index);

                self.buf_interval = Duration::ZERO;
            } else {
                self.buf_interval += interval;
            }
        }
        {
            let pre_activity = &self.stat.activity;
            let no_process_using_vram = self.stat.fdinfo.no_process_using_vram();

            // TODO: those checks may not be enough
            if no_process_using_vram
                && self.amdgpu_dev.is_some()
                && !self.no_drop_device_handle
                && !self.device_info.is_apu
                && pre_activity.is_all_idling()
            {
                {
                    let dev = self.amdgpu_dev.as_ref().unwrap();
                    self.stat.vram_usage.update_usage(dev);
                    self.stat.vram_usage.update_usable_heap_size(dev);
                }

                unsafe { ManuallyDrop::drop(&mut self.amdgpu_dev); }
                self.amdgpu_dev = ManuallyDrop::new(None);
            } else if !no_process_using_vram
                && self.amdgpu_dev.is_none()
                && self.device_path.check_if_device_is_active()
            {
                self.amdgpu_dev = ManuallyDrop::new(self.device_path.init().ok());
            }

            self.dynamic_no_pc = no_process_using_vram;
        }

        if self.amdgpu_dev.is_none() {
            if let Some(ref mut sensors) = self.stat.sensors {
                sensors.update_for_idle();
            }

            self.stat.metrics = None;
            return;
        };

        let was_idle = self.stat.sensors.as_ref().is_some_and(|s| s.is_idle);

        if self.stat.metrics.is_some() || (self.stat.metrics.is_none() && was_idle) {
            self.stat.metrics = GpuMetrics::get_from_sysfs_path(&self.device_info.sysfs_path).ok();
        }

        if let Some(dev) = self.amdgpu_dev.as_ref() {
            self.stat.vram_usage.update_usage(dev);
            self.stat.vram_usage.update_usable_heap_size(dev);

            if let Some(ref mut sensors) = self.stat.sensors {
                sensors.update(dev);
            } else {
                self.stat.sensors = Sensors::new(
                    dev,
                    &self.device_info.pci_bus,
                    &self.device_info.ext_info,
                );
            }
        }

        if self.device_info.smu_ip_version.is_some_and(|ip_ver| ip_ver == (13, 0, 5))
        {
            self.stat.workaround_for_smu_v13_0_5();
        }

        if self.device_info.smu_ip_version
            .is_some_and(|ip_ver| ip_ver == (14, 0, 2) || ip_ver == (14, 0, 3))
        {
            self.stat.workaround_fan_rpm_for_smu_v14();
        }

        if self.stat.memory_error_count.is_some() {
            self.stat.memory_error_count = RasErrorCount::get_from_sysfs_with_ras_block(
                &self.device_info.sysfs_path,
                RasBlock::UMC,
            ).ok();
        }

        self.stat.activity = GpuActivity::get_with_option_gpu_metrics(
            &self.device_info.sysfs_path,
            self.device_info.asic_name,
            &self.stat.metrics,
        );

        if self.stat.activity.media.is_none() || self.stat.activity.media == Some(0) {
            let (proc_usage, _, _, _) = self.stat.fdinfo.fold_fdinfo_usage();
            self.stat.activity.media = proc_usage.media.try_into().ok();
        }
    }

    pub fn update_pc(&mut self) {
        if self.dynamic_no_pc { return }

        if let Some(dev) = self.amdgpu_dev.as_ref() {
            self.stat.grbm.read_reg(dev);
            self.stat.grbm2.read_reg(dev);
        }
    }

    pub fn clear_pc(&mut self) {
        self.stat.grbm.clear_pc();
        self.stat.grbm2.clear_pc();
    }

    pub fn update_pc_usage(&mut self) {
        self.stat.grbm.update_pc_usage();
        self.stat.grbm2.update_pc_usage();
    }

    pub fn get_drm_version_struct(&mut self) -> Option<drmVersion> {
        self.amdgpu_dev
            .as_ref()
            .and_then(|dev| dev.get_drm_version_struct().ok())
    }
}
