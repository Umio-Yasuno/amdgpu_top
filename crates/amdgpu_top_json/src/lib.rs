use libamdgpu_top::{DevicePath, stat};
use libamdgpu_top::app::*;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::io::Write;

mod output_json;
use crate::output_json::FdInfoJson;
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
    pub sus_app_list: Vec<DevicePath>,
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
            JsonDeviceInfo::from_device_path_list(device_path_list);

        for device in vec_device_info.iter_mut() {
            device.app.stat.fdinfo.interval = interval;
            device.app.update(interval);
        }

        let base_time = Instant::now();
        let duration_time = base_time.elapsed();

        {
            let mut device_paths: Vec<DevicePath> = device_path_list.to_vec();

            if let Some(xdna_device_path) = vec_device_info
                .iter()
                .find_map(|j| j.app.xdna_device_path.as_ref())
            {
                device_paths.push(xdna_device_path.clone());
            }

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

    pub fn update(&mut self) {
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

            for device in self.vec_device_info.iter_mut() {
                device.app.update_pc_usage();
            }
        } else {
            std::thread::sleep(self.delay * 100);
        }

        for device in self.vec_device_info.iter_mut() {
            device.app.update(self.interval);
        }

        self.sus_app_list.retain(|sus_device| {
            let is_active = sus_device.check_if_device_is_active();

            if is_active {
                let Some(amdgpu_dev) = sus_device.init().ok() else { return true };
                let Some(mut app) = AppAmdgpuTop::new(
                    amdgpu_dev,
                    sus_device.clone(),
                    &Default::default(),
                ) else { return true };
                let info = app.json_info();
                self.vec_device_info.push(JsonDeviceInfo { app, info });
            }

            !is_active
        });

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
            .map(|sus_dev| sus_dev.json())
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

        loop {
            self.update();

            let s = self.json().to_string();

            println!("{s}");

            if self.iterations != 0 {
                n += 1;
                if self.iterations == n { break; }
            }
        }
    }

    pub fn run_fifo(&mut self, fifo_path: PathBuf) {
        loop {
            self.update();

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
    pub fn from_device_path_list(device_path_list: &[DevicePath]) -> (
        Vec<Self>,
        Vec<DevicePath>,
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
        let (proc_usage, has_vcn, has_vcn_unified, has_vpe) =
            self.app.stat.fdinfo.fold_fdinfo_usage();

        json!({
            "Info": self.info,
            "GRBM": if !no_pc { self.app.stat.grbm.json() } else { Value::Null },
            "GRBM2": if !no_pc { self.app.stat.grbm2.json() } else { Value::Null },
            "VRAM": self.app.stat.vram_usage.json(),
            "Sensors": self.app.stat.sensors.as_ref().map(|s| s.json()),
            "fdinfo": self.app.stat.fdinfo.json(),
            "xdna_fdinfo": self.app.stat.xdna_fdinfo.json(),
            "Total fdinfo": proc_usage.usage_json(has_vcn, has_vcn_unified, has_vpe),
            "gpu_metrics": self.app.stat.metrics.as_ref().map(|m| m.json()),
            "gpu_activity": self.app.stat.activity.json(),
        })
    }
}
