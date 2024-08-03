use libamdgpu_top::{DevicePath, stat, PCI};
use libamdgpu_top::app::*;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::io::Write;
use std::collections::HashMap;

mod output_json;
mod dump;
pub use dump::{dump_json, drm_info_json, gpu_metrics_json, JsonInfo};

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
    pub sus_app_list: HashMap<PCI::BUS_INFO, DevicePath>,
    pub base_time: Instant,
    pub interval: Duration,
    pub duration_time: Duration,
    pub delay: Duration,
    pub iterations: u32,
    pub no_pc: bool,
    pub amdgpu_top_version: Value,
    pub rocm_version: Value,
    pub title: String,
}

impl JsonApp {
    pub fn new(
        title: &str,
        device_path_list: &[DevicePath],
        refresh_period: u64,
        update_process_index_interval: u64,
        iterations: u32,
        no_pc: bool,
    ) -> Self {
        let interval = Duration::from_millis(refresh_period);
        let delay = interval / 100;
        let (mut vec_device_info, sus_app_list) =
            JsonDeviceInfo::from2_device_path_list(device_path_list);

        for device in vec_device_info.iter_mut() {
            device.app.stat.fdinfo.interval = interval;
            device.app.update(interval);
        }

        let base_time = Instant::now();
        let duration_time = base_time.elapsed();

        {
            let device_paths: Vec<DevicePath> = device_path_list.to_vec();
            stat::spawn_update_index_thread(device_paths, update_process_index_interval);
        }

        Self {
            vec_device_info,
            sus_app_list,
            base_time,
            duration_time,
            interval,
            delay,
            iterations,
            no_pc,
            amdgpu_top_version: amdgpu_top_version(),
            rocm_version: libamdgpu_top::get_rocm_version().map_or(Value::Null, Value::String),
            title: title.to_string(),
        }
    }

    pub fn update(&mut self, remove_sus_devices: &mut Vec<PCI::BUS_INFO>) {
        for pci in remove_sus_devices.iter() {
            let _ = self.sus_app_list.remove(pci);
        }

        if !remove_sus_devices.is_empty() {
            remove_sus_devices.clear();
            remove_sus_devices.shrink_to_fit();
        }

        if !self.no_pc {
            for device in self.vec_device_info.iter_mut() {
                device.app.clear_pc();
            }
        }

        if !self.no_pc {
            for _ in 0..100 {
                for device in self.vec_device_info.iter_mut() {
                    device.app.update_pc();
                }
                std::thread::sleep(self.delay);
            }
        } else {
            std::thread::sleep(self.delay * 100);
        }

        for device in self.vec_device_info.iter_mut() {
            device.app.update(self.interval);
        }

        for (pci, sus_device) in self.sus_app_list.iter() {
            if sus_device.check_if_device_is_active() {
                let Some(amdgpu_dev) = sus_device.init().ok() else { continue };
                let Some(mut app) = AppAmdgpuTop::new(
                    amdgpu_dev,
                    sus_device.clone(),
                    &Default::default(),
                ) else { continue };
                let info = app.json_info();
                self.vec_device_info.push(JsonDeviceInfo { app, info });
                remove_sus_devices.push(*pci);
            }
        }

        self.duration_time = {
            let now = Instant::now();
            now.duration_since(self.base_time)
        };
    }

    pub fn json(&self) -> Value {
        let devices: Vec<Value> = self.vec_device_info
            .iter()
            .map(|device| device.json(self.no_pc))
            .collect();
        let sus_devices: Vec<Value> = self.sus_app_list
            .iter()
            .map(|(_pci, sus_dev)| sus_dev.json())
            .collect();

        json!({
            "period": {
                "duration": self.duration_time.as_millis(),
                "unit": "ms",
            },
            "devices": devices,
            "suspended_devices": sus_devices,
            "devices_len": devices.len(),
            "suspended_devices_len": sus_devices.len(),
            "amdgpu_top_version": self.amdgpu_top_version,
            "ROCm version": self.rocm_version,
            "title": self.title,
        })
    }

    pub fn run(&mut self) {
        let mut n = 0;
        let mut remove_sus_devices = Vec::new();

        loop {
            self.update(&mut remove_sus_devices);

            let s = self.json().to_string();

            println!("{s}");

            if self.iterations != 0 {
                n += 1;
                if self.iterations == n { break; }
            }
        }
    }

    pub fn run_fifo(&mut self, fifo_path: PathBuf) {
        let mut remove_sus_devices = Vec::new();

        loop {
            self.update(&mut remove_sus_devices);

            let s = self.json().to_string();

            let mut f = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&fifo_path)
                .unwrap();

            f.write_all(s.as_bytes()).unwrap();
            f.flush().unwrap();
        }
    }
}

pub struct JsonDeviceInfo {
    pub app: AppAmdgpuTop,
    pub info: Value,
}

impl JsonDeviceInfo {
    pub fn from_device_path_list(device_path_list: &[DevicePath]) -> Vec<Self> {
        let vec_json_device: Vec<Self> = device_path_list.iter().filter_map(|device_path| {
            let amdgpu_dev = device_path.init().ok()?;
            let mut app = AppAmdgpuTop::new(amdgpu_dev, device_path.clone(), &Default::default())?;
            let info = app.json_info();

            Some(Self { app, info })
        }).collect();

        vec_json_device
    }

    pub fn from2_device_path_list(device_path_list: &[DevicePath]) -> (
        Vec<Self>,
        HashMap<PCI::BUS_INFO, DevicePath>,
    ) {
        let (vec_app, sus_app_list) = AppAmdgpuTop::create_app_and_suspended_list(
            device_path_list,
            &Default::default(),
        );
        let vec_json_device = vec_app
            .into_iter()
            .map(|mut app| {
                let info = app.json_info();

                Self { app, info }
            })
            .collect();

        (vec_json_device, sus_app_list)
    }

    pub fn json(&self, no_pc: bool) -> Value {
        json!({
            "Info": self.info,
            "GRBM": if !no_pc { self.app.stat.grbm.json() } else { Value::Null },
            "GRBM2": if !no_pc { self.app.stat.grbm2.json() } else { Value::Null },
            "VRAM": self.app.stat.vram_usage.json(),
            "Sensors": self.app.stat.sensors.as_ref().map(|s| s.json()),
            "fdinfo": self.app.stat.fdinfo.json(),
            "Total fdinfo": self.app.stat.fdinfo.fold_fdinfo_usage().json(),
            "gpu_metrics": self.app.stat.metrics.as_ref().map(|m| m.json()),
            "gpu_activity": self.app.stat.activity.json(),
        })
    }
}
