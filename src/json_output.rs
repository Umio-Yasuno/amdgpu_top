use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, CHIP_CLASS, GPU_INFO};
use crate::stat;
use std::time::{Duration, Instant};
use std::io::{self, stdin, stdout, Read, Write, BufReader, BufWriter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ctrlc;

pub fn print(
    amdgpu_dev: &DeviceHandle,
    device_path: &str,
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
    let mut cp_stat = stat::PerfCounter::new(stat::PCType::CP_STAT, stat::CP_STAT_INDEX);
    let mut vram = stat::VRAM_INFO::new(&memory_info);

    let mut period = Duration::from_millis(refresh_period);
    let delay = period / 100;

    let proc_info = stat::ProcInfo::from_pid(pid, device_path);
    let mut fdinfo = stat::FdInfoView::new(period);
    fdinfo.get_proc_usage(&proc_info);

    let mut sensor = stat::Sensor::new(&pci_bus);

    let out = stdout();
    let mut out = BufWriter::new(out.lock());
    let quit_flag = Arc::new(AtomicBool::new(false));

    {
        let mut stdin = BufReader::new(stdin());
        let mut buf = [0u8; 1];
        let quit_flag = quit_flag.clone();

        std::thread::spawn(move || {
            loop {
                if let Err(_) = stdin.read(&mut buf[..]) {
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

    out.write_all(b"[\n")?;

    let pad = "\"\": {}".to_string();
    let base = Instant::now();

    loop {
        for _ in 0..100 {
            grbm.read_reg(amdgpu_dev);
            grbm2.read_reg(amdgpu_dev);
            cp_stat.read_reg(amdgpu_dev);

            std::thread::sleep(delay);
        }

        vram.update_usage(amdgpu_dev);
        sensor.update_status();
        fdinfo.proc_usage.clear();
        fdinfo.get_proc_usage(&proc_info);

        let now = Instant::now();
        period = now.duration_since(base);

        out.write_all(b"{\n")?;

        write!(
            out,
            concat!(
                "\t\"Device Name\": \"{name}\",\n",
                "\t\"CU Count\": {cu_count},\n",
                "\t\"ResizableBAR\": {rebar},\n",
            ),
            name = mark_name,
            cu_count = cu_count,
            rebar = resizable_bar,
        )?;
        write!(
            out,
            concat!(
                "\t\"period\": {{\n",
                "\t\t\"duration\": {duration},\n",
                "\t\t\"unit\": \"ms\"\n",
                "\t}},\n",
            ),
            duration = period.as_millis(),
        )?;
        write!(
            out,
            concat!(
                "{grbm},\n",
                "{grbm2},\n",
                "{cp_stat},\n",
                "{vram},\n",
                "{fdinfo},\n",
                "{sensor}\n",
            ),
            grbm = grbm.json().unwrap_or(pad.clone()),
            grbm2 = grbm2.json().unwrap_or(pad.clone()),
            cp_stat = cp_stat.json().unwrap_or(pad.clone()),
            vram = vram.json().unwrap_or(pad.clone()),
            fdinfo = fdinfo.json().unwrap_or(pad.clone()),
            sensor = sensor.json(amdgpu_dev).unwrap_or(pad.clone()),
        )?;

        grbm.bits.clear();
        grbm2.bits.clear();
        cp_stat.bits.clear();

        if quit_flag.load(Ordering::Relaxed) {
            out.write_all(b"}\n]\n")?;
            return Ok(());
        } else {
            out.write_all(b"},\n")?;
        }

        out.flush()?;
    }
}
