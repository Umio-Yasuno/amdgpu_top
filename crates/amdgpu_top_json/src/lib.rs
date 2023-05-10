use libamdgpu_top::AMDGPU::{DeviceHandle, CHIP_CLASS, GPU_INFO};
use libamdgpu_top::{DevicePath, stat, VramUsage};
use stat::{FdInfoStat, Sensors, PerfCounter};
use serde_json::json;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

mod output_json;
use output_json::OutputJson;

pub fn run(
    device_path: DevicePath,
    amdgpu_dev: DeviceHandle,
    refresh_period: u64,
    update_process_index: u64,
) {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let chip_class = ext_info.get_chip_class();
    let mark_name = amdgpu_dev.get_marketing_name().unwrap_or("".to_string());
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();

    let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
        stat::GFX10_GRBM_INDEX
    } else {
        stat::GRBM_INDEX
    };

    let mut grbm = PerfCounter::new(stat::PCType::GRBM, grbm_index);
    let mut grbm2 = PerfCounter::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);
    let mut vram = VramUsage::new(&memory_info);
    let mut sensors = Sensors::new(&amdgpu_dev, &pci_bus);

    let mut period = Duration::from_millis(refresh_period);
    let interval = period.clone();
    let delay = period / 100;

    let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
    let mut fdinfo = FdInfoStat::new(interval);
    {
        stat::update_index(&mut proc_index, &device_path);
        for pu in &proc_index {
            fdinfo.get_proc_usage(pu);
        }
    }

    let share_proc_index = Arc::new(Mutex::new(proc_index));
    {
        let index = share_proc_index.clone();
        let mut buf_index: Vec<stat::ProcInfo> = Vec::new();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(update_process_index));

            stat::update_index(&mut buf_index, &device_path);

            let lock = index.lock();
            if let Ok(mut index) = lock {
                *index = buf_index.clone();
            }
        });
    }

    let base = Instant::now();

    loop {
        for _ in 0..100 {
            grbm.read_reg(&amdgpu_dev);
            grbm2.read_reg(&amdgpu_dev);

            std::thread::sleep(delay);
        }

        vram.update_usage(&amdgpu_dev);
        sensors.update(&amdgpu_dev);

        {
            let lock = share_proc_index.try_lock();
            if let Ok(proc_index) = lock {
                fdinfo.get_all_proc_usage(&proc_index);
                fdinfo.interval = interval;
            } else {
                fdinfo.interval += interval;
            }
        }

        let now = Instant::now();
        period = now.duration_since(base);

        let json = json!({
            "DeviceName": mark_name,
            "period": {
                "duration": period.as_millis(),
                "unit": "ms",
            },
            "GRBM": grbm.json(),
            "GRBM2": grbm2.json(),
            "VRAM": vram.json(),
            "Sensors": sensors.json(),
            "fdinfo": fdinfo.json(),
        });

        grbm.bits.clear();
        grbm2.bits.clear();

        println!("{json}");
    }
}
