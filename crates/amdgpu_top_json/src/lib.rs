use libamdgpu_top::AMDGPU::{ASIC_NAME, DeviceHandle, GPU_INFO, GpuMetrics};
use libamdgpu_top::{DevicePath, stat, VramUsage};
use stat::{FdInfoStat, GpuActivity, Sensors, PerfCounter, ProcInfo};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

mod output_json;
mod dump;
pub use dump::{dump_json, json_info};

pub fn version_json(title: &str) {
    let version = json!({
        "version": amdgpu_top_version(),
        "title": title,
    });
    println!("{version}");
}

pub trait OutputJson {
    fn json(&self) -> Value;
}

pub fn amdgpu_top_version() -> Value {
    json!({
        "major": env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap_or(0),
        "minor": env!("CARGO_PKG_VERSION_MINOR").parse::<u32>().unwrap_or(0),
        "patch": env!("CARGO_PKG_VERSION_PATCH").parse::<u32>().unwrap_or(0),
    })
}

pub struct JsonApp {
    pub vec_device_info: Vec<JsonDeviceInfo>,
    pub base_time: Instant,
    pub period: Duration,
    pub interval: Duration,
    pub delay: Duration,
    pub iterations: u32,
}

impl JsonApp {
    pub fn new(
        device_path_list: &[DevicePath],
        refresh_period: u64,
        update_process_index_interval: u64,
        iterations: u32,
    ) -> Self {
        let period = Duration::from_millis(refresh_period);
        let interval = period.clone();
        let delay = period / 100;
        let mut vec_device_info = JsonDeviceInfo::from_device_path_list(device_path_list);

        for device in vec_device_info.iter_mut() {
            device.fdinfo.interval = interval;
            device.update(interval);
        }

        let base_time = Instant::now();

        {
            let t_index: Vec<(DevicePath, Arc<Mutex<Vec<ProcInfo>>>)> = vec_device_info
                .iter()
                .map(|device| (device.device_path.clone(), device.arc_proc_index.clone()))
                .collect();
            stat::spawn_update_index_thread(t_index, update_process_index_interval);
        }

        Self {
            vec_device_info,
            base_time,
            period,
            interval,
            delay,
            iterations,
        }
    }

    pub fn run(&mut self, title: &str) {
        let mut n = 0;
        let mut buf_json: Vec<Value> = Vec::with_capacity(self.vec_device_info.len());
        let devices_len = self.vec_device_info.len();

        loop {
            for _ in 0..100 {
                for device in self.vec_device_info.iter_mut() {
                    device.update_pc();
                }
                std::thread::sleep(self.delay);
            }

            for device in self.vec_device_info.iter_mut() {
                device.update(self.interval);

                buf_json.push(device.json());

                device.clear_pc();
            }

            let now = Instant::now();

            println!("{}", json!({
                "period": {
                    "duration": now.duration_since(self.base_time).as_millis(),
                    "unit": "ms",
                },
                "devices": Value::Array(buf_json.clone()),
                "devices_len": devices_len,
                "amdgpu_top_version": amdgpu_top_version(),
                "title": title,
            }));

            buf_json.clear();

            if self.iterations != 0 {
                n += 1;
                if self.iterations == n { break; }
            }
        }
    }
}

pub struct JsonDeviceInfo {
    pub amdgpu_dev: DeviceHandle,
    pub device_path: DevicePath,
    pub info: Value,
    // pub pci_bus:
    pub asic_name: ASIC_NAME,
    pub grbm: PerfCounter,
    pub grbm2: PerfCounter,
    pub vram_usage: VramUsage,
    pub sensors: Sensors,
    pub sysfs_path: PathBuf,
    pub metrics: Option<GpuMetrics>,
    pub activity: GpuActivity,
    pub fdinfo: FdInfoStat,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
}

impl JsonDeviceInfo {
    pub fn from_device_path_list(device_path_list: &[DevicePath]) -> Vec<Self> {
        let vec_json_device: Vec<Self> = device_path_list.iter().filter_map(|device_path| {
            let amdgpu_dev = device_path.init().ok()?;

            Self::new(amdgpu_dev, device_path.clone())
        }).collect();

        vec_json_device
    }

    pub fn new(amdgpu_dev: DeviceHandle, device_path: DevicePath) -> Option<Self> {
        let pci_bus = amdgpu_dev.get_pci_bus_info().ok()?;
        let ext_info = amdgpu_dev.device_info().ok()?;
        let asic_name = ext_info.get_asic_name();
        let memory_info = amdgpu_dev.memory_info().ok()?;
        let info = json_info(&amdgpu_dev, &pci_bus, &ext_info, &memory_info);
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
        let fdinfo = FdInfoStat {
            has_vcn: libamdgpu_top::has_vcn(&amdgpu_dev),
            has_vcn_unified: libamdgpu_top::has_vcn_unified(&amdgpu_dev),
            ..Default::default()
        };

        Some(Self {
            amdgpu_dev,
            device_path,
            info,
            asic_name,
            grbm,
            grbm2,
            vram_usage,
            sensors,
            metrics,
            activity,
            sysfs_path,
            fdinfo,
            arc_proc_index,
        })
    }

    pub fn update(&mut self, interval: Duration) {
        self.vram_usage.update_usage(&self.amdgpu_dev);
        self.sensors.update(&self.amdgpu_dev);
        self.metrics = self.amdgpu_dev.get_gpu_metrics_from_sysfs_path(&self.sysfs_path).ok();
        self.activity = GpuActivity::get(&self.amdgpu_dev, &self.sysfs_path, self.asic_name);

        {
            let lock = self.arc_proc_index.try_lock();
            if let Ok(proc_index) = lock {
                self.fdinfo.get_all_proc_usage(&proc_index);
                self.fdinfo.interval = interval;
            } else {
                self.fdinfo.interval += interval;
            }
        }

        if self.activity.media.is_none() || self.activity.media == Some(0) {
            self.activity.media = self.fdinfo.fold_fdinfo_usage().media.try_into().ok();
        }
    }

    pub fn update_pc(&mut self) {
        self.grbm.read_reg(&self.amdgpu_dev);
        self.grbm2.read_reg(&self.amdgpu_dev);
    }

    pub fn clear_pc(&mut self) {
        self.grbm.bits.clear();
        self.grbm2.bits.clear();
    }
}

impl OutputJson for JsonDeviceInfo {
    fn json(&self) -> Value {
        json!({
            "Info": self.info,
            "GRBM": self.grbm.json(),
            "GRBM2": self.grbm2.json(),
            "VRAM": self.vram_usage.json(),
            "Sensors": self.sensors.json(),
            "fdinfo": self.fdinfo.json(),
            "gpu_metrics": self.metrics.as_ref().map(|m| m.json()),
            "gpu_activity": self.activity.json(),
        })
    }
}
