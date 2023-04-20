use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, CHIP_CLASS, GPU_INFO};
use crate::{DevicePath, stat};
use std::time::{Duration, Instant};
use std::io::{self, stdin, Read, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde_json::{json, Value};

pub fn print(
    amdgpu_dev: &DeviceHandle,
    device_path: &DevicePath,
    refresh_period: u64,
    pid: i32
) -> io::Result<()> {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let chip_class = ext_info.get_chip_class();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let mark_name = amdgpu_dev.get_marketing_name().unwrap_or("".to_string());
    let cu_count = ext_info.cu_active_number();

    /* ref: https://gitlab.freedesktop.org/mesa/mesa/blob/main/src/amd/common/ac_gpu_info.c */
    let resizable_bar = (memory_info.vram.total_heap_size * 9 / 10) <= memory_info.cpu_accessible_vram.total_heap_size;

    let grbm_index = if CHIP_CLASS::GFX10 <= chip_class {
        stat::GFX10_GRBM_INDEX
    } else {
        stat::GRBM_INDEX
    };

    let mut grbm = stat::PerfCounter::new(stat::PCType::GRBM, grbm_index);
    let mut grbm2 = stat::PerfCounter::new(stat::PCType::GRBM2, stat::GRBM2_INDEX);
    let mut vram = stat::VramUsageView::new(&memory_info);

    let mut period = Duration::from_millis(refresh_period);
    let delay = period / 100;

    let proc_info = stat::ProcInfo::from_pid(pid, device_path);
    let mut fdinfo = stat::FdInfoView::new(period);
    fdinfo.get_proc_usage(&proc_info);

    let mut sensor = stat::Sensor::new(&pci_bus);

    let quit_flag = Arc::new(AtomicBool::new(false));

    {
        let mut stdin = BufReader::new(stdin());
        let mut buf = [0u8; 1];
        let quit_flag = quit_flag.clone();

        std::thread::spawn(move || {
            loop {
                if stdin.read(&mut buf[..]).is_err() {
                    eprintln!("Read error");
                    quit_flag.store(true, Ordering::Relaxed);
                    return;
                };
                if b"q" == &buf || b"Q" == &buf {
                    quit_flag.store(true, Ordering::Relaxed);
                }
            }
        });
    }

    {
        let quit_flag = quit_flag.clone();
        ctrlc::set_handler(move || quit_flag.store(true, Ordering::Relaxed))
            .expect("Error setting Ctrl-C handler");
    }

    let mut vec_value: Vec<Value> = Vec::new();
    let base = Instant::now();

    loop {
        for _ in 0..100 {
            grbm.read_reg(amdgpu_dev);
            grbm2.read_reg(amdgpu_dev);

            std::thread::sleep(delay);
        }

        vram.update_usage(amdgpu_dev);
        sensor.update_status();
        fdinfo.proc_usage.clear();
        fdinfo.get_proc_usage(&proc_info);

        let now = Instant::now();
        period = now.duration_since(base);

        let json = json!({
            "DeviceName": mark_name,
            "ResizableBar": resizable_bar,
            "CU Count": cu_count,
            "period": {
                "duration": period.as_millis(),
                "unit": "ms",
            },
            "GRBM": grbm.json_value(),
            "GRBM2": grbm2.json_value(),
            "VRAM": vram.json_value(),
            "fdinfo": fdinfo.json_value(),
            "Sensors": sensor.json_value(amdgpu_dev),
        });

        grbm.bits.clear();
        grbm2.bits.clear();

        vec_value.push(json);

        if quit_flag.load(Ordering::Relaxed) {
            println!("{}", vec_value.into_iter().collect::<Value>());
            return Ok(());
        }
    }
}
