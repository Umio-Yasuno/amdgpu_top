use libamdgpu_top::{DevicePath, stat};
use libamdgpu_top::app::*;
use stat::{ProcInfo};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::io::Write;

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
        let mut vec_device_info = JsonDeviceInfo::from_device_path_list(device_path_list);

        for device in vec_device_info.iter_mut() {
            device.app.stat.fdinfo.interval = interval;
            device.app.update(interval);
        }

        let base_time = Instant::now();
        let duration_time = base_time.elapsed();

        {
            let t_index: Vec<(DevicePath, Arc<Mutex<Vec<ProcInfo>>>)> = vec_device_info
                .iter()
                .map(|device| (
                    device.app.device_path.clone(),
                    device.app.stat.arc_proc_index.clone(),
                ))
                .collect();
            stat::spawn_update_index_thread(t_index, update_process_index_interval);
        }

        Self {
            vec_device_info,
            base_time,
            duration_time,
            interval,
            delay,
            iterations,
            no_pc,
            amdgpu_top_version: amdgpu_top_version(),
            rocm_version: libamdgpu_top::get_rocm_version().map_or(Value::Null, |ver| Value::String(ver)),
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
        } else {
            std::thread::sleep(self.delay * 100);
        }

        for device in self.vec_device_info.iter_mut() {
            device.app.update(self.interval);
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

        json!({
            "period": {
                "duration": self.duration_time.as_millis(),
                "unit": "ms",
            },
            "devices": devices,
            "devices_len": self.vec_device_info.len(),
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
    pub fn from_device_path_list(device_path_list: &[DevicePath]) -> Vec<Self> {
        let vec_json_device: Vec<Self> = device_path_list.iter().filter_map(|device_path| {
            let amdgpu_dev = device_path.init().ok()?;
            let app = AppAmdgpuTop::new(amdgpu_dev, device_path.clone(), &Default::default())?;
            let info = app.json_info();

            Some(Self { app, info })
        }).collect();

        vec_json_device
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
