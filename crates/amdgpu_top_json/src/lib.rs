use libamdgpu_top::{DevicePath, stat};
use libamdgpu_top::app::*;
use stat::{ProcInfo};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

mod output_json;
mod dump;
pub use dump::{dump_json, JsonInfo};

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
    pub no_pc: bool,
}

impl JsonApp {
    pub fn new(
        device_path_list: &[DevicePath],
        refresh_period: u64,
        update_process_index_interval: u64,
        iterations: u32,
        no_pc: bool,
    ) -> Self {
        let period = Duration::from_millis(refresh_period);
        let interval = period.clone();
        let delay = period / 100;
        let mut vec_device_info = JsonDeviceInfo::from_device_path_list(device_path_list);

        for device in vec_device_info.iter_mut() {
            device.app.stat.fdinfo.interval = interval;
            device.app.update(interval);
        }

        let base_time = Instant::now();

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
            period,
            interval,
            delay,
            iterations,
            no_pc,
        }
    }

    pub fn run(&mut self, title: &str) {
        let mut n = 0;
        let mut buf_json: Vec<Value> = Vec::with_capacity(self.vec_device_info.len());
        let devices_len = self.vec_device_info.len();

        loop {
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

                buf_json.push(device.json(self.no_pc));
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
            "GRBM": self.app.stat.grbm.json(),
            "GRBM2": if !no_pc { self.app.stat.grbm2.json() } else { Value::Null },
            "VRAM": if !no_pc { self.app.stat.vram_usage.json() } else { Value::Null },
            "Sensors": self.app.stat.sensors.json(),
            "fdinfo": self.app.stat.fdinfo.json(),
            "gpu_metrics": self.app.stat.metrics.as_ref().map(|m| m.json()),
            "gpu_activity": self.app.stat.activity.json(),
        })
    }
}
