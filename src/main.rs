use cursive::views::{TextContent, TextView, LinearLayout};
use libdrm_amdgpu_sys::*;
use AMDGPU::GPU_INFO;
use std::sync::{Arc, Mutex};

mod grbm;
use grbm::*;

mod srbm;
use srbm::*;

mod srbm2;
use srbm2::*;

mod cp_stat;
use cp_stat::*;

mod vram_usage;
use vram_usage::*;

mod sensors;
use sensors::*;

#[derive(Debug, Clone)]
struct UserOptions {
    grbm: bool,
    uvd: bool,
    srbm: bool,
    cp_stat: bool,
    vram: bool,
    sensor: bool,
    high_freq: bool,
}

impl Default for UserOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            uvd: true,
            srbm: true,
            cp_stat: false,
            vram: true,
            sensor: true,
            high_freq: false,
        }
    }
}

struct Sampling {
    count: usize,
    delay: std::time::Duration,
}

impl Sampling {
    const fn low() -> Self {
        Self {
            count: 100,
            delay: std::time::Duration::from_millis(10),
        }
    }
    const fn high() -> Self {
        Self {
            count: 100,
            delay: std::time::Duration::from_millis(1),
        }
    }
}

const TOGGLE_HELP: &str = " v(g)t (u)vd (s)rbm (c)p_stat\n (v)ram se(n)sor (h)igh_freq (q)uit";

fn main() {
    let (amdgpu_dev, _major, _minor) = {
        use std::fs::File;
        use std::os::fd::IntoRawFd;

        let fd = File::open("/dev/dri/renderD128").unwrap();

        AMDGPU::DeviceHandle::init(fd.into_raw_fd()).unwrap()
    };
    let ext_info = amdgpu_dev.device_info().unwrap();

    let mut grbm = GRBM::new();
    let mut srbm = SRBM::new();
    let mut srbm2 = SRBM2::new();
    let mut cp_stat = CP_STAT::new();

    let grbm_offset = ext_info.get_grbm_offset();
    let srbm_offset = ext_info.get_srbm_offset();
    let srbm2_offset = ext_info.get_srbm2_offset();
    let cp_stat_offset = ext_info.get_cp_stat_offset();

    // check register offset
    check_register_offset(&amdgpu_dev, "mmGRBM_STATUS", grbm_offset);
    check_register_offset(&amdgpu_dev, "mmSRBM_STATUS", srbm_offset);
    check_register_offset(&amdgpu_dev, "mmSRBM_STATUS2", srbm2_offset);
    check_register_offset(&amdgpu_dev, "mmCP_STAT", cp_stat_offset);

    let grbm_view = TextContent::new(grbm.verbose_stat()); 
    let srbm_view = TextContent::new(srbm.stat());
    let srbm2_view = TextContent::new(srbm2.stat());
    let sensor_view = TextContent::new(Sensor::stat(&amdgpu_dev));
    let cp_stat_view = TextContent::new("");
    let vram_view = TextContent::new(
        if let Ok(info) = amdgpu_dev.memory_info() {
            VRAM_USAGE::from(info).stat()
        } else {
            "".to_string()
        }
    );

    let [min_gpu_clk, min_memory_clk] = {
        if let Ok(pci_bus) = amdgpu_dev.get_pci_bus_info() {
            if let [Some(gpu), Some(mem)] = [
                amdgpu_dev.get_min_gpu_clock_from_sysfs(&pci_bus),
                amdgpu_dev.get_min_memory_clock_from_sysfs(&pci_bus),
            ] {
                [gpu, mem]
            } else {
                [0, 0]
            }
        } else {
            [0, 0]
        }
    };

    let info_bar = format!(
        concat!(
            "{asic}, {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
            "{vram_type} {vram_bus_width}-bit, {min_memory_clk}-{max_memory_clk} MHz",
        ),
        asic = ext_info.get_asic_name(),
        num_cu = ext_info.cu_active_number(),
        min_gpu_clk = min_gpu_clk,
        max_gpu_clk = ext_info.max_engine_clock().saturating_div(1000),
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
        min_memory_clk = min_memory_clk,
        max_memory_clk = ext_info.max_memory_clock().saturating_div(1000),
    );
    let mark_name = match amdgpu_dev.get_marketing_name() {
        Ok(name) => name,
        Err(_) => "".to_string(),
    };
    let user_opt = Arc::new(Mutex::new(UserOptions::default()));

    let mut siv = cursive::default();

    siv.add_layer(
        LinearLayout::vertical()
            .child(TextView::new(format!(" amdgpu_top @ {mark_name} ")).center())
            .child(TextView::new(info_bar).center())
            .child(TextView::new_with_content(grbm_view.clone()).center())
            .child(TextView::new_with_content(srbm_view.clone()).center())
            .child(TextView::new_with_content(srbm2_view.clone()).center())
            .child(TextView::new_with_content(cp_stat_view.clone()).center())
            .child(TextView::new_with_content(vram_view.clone()).center())
            .child(TextView::new_with_content(sensor_view.clone()).center())
            .child(TextView::new(TOGGLE_HELP))
    );
    set_global_cb(&mut siv);
    siv.set_user_data(user_opt.clone());

    let cb_sink = siv.cb_sink().clone();
    let opt = user_opt.clone();

    let mut sample = Sampling::low();

    std::thread::spawn(move || {
        loop {
            for _ in 0..sample.count {
                if let Ok(out) = amdgpu_dev.read_mm_registers(grbm_offset) {
                    grbm.acc(out);
                }
                if let Ok(out) = amdgpu_dev.read_mm_registers(srbm_offset) {
                    srbm.acc(out);
                }
                if let Ok(out) = amdgpu_dev.read_mm_registers(srbm2_offset) {
                    srbm2.acc(out);
                }
                if let Ok(out) = amdgpu_dev.read_mm_registers(cp_stat_offset) {
                    cp_stat.acc(out);
                }
                std::thread::sleep(sample.delay);
            }

            if let Ok(opt) = opt.try_lock() {
                if opt.grbm {
                    grbm_view.set_content(grbm.verbose_stat());
                } else {
                    grbm_view.set_content("");
                }

                if opt.uvd {
                    srbm_view.set_content(srbm.stat());
                } else {
                    srbm_view.set_content("");
                }

                if opt.srbm {
                    srbm2_view.set_content(srbm2.stat());
                } else {
                    srbm2_view.set_content("");
                }

                if opt.cp_stat {
                    cp_stat_view.set_content(cp_stat.verbose_stat());
                } else {
                    cp_stat_view.set_content("");
                }

                if opt.vram {
                    if let Ok(info) = amdgpu_dev.memory_info() {
                        vram_view.set_content(VRAM_USAGE::from(info).stat());
                    }
                } else {
                    vram_view.set_content("");
                }

                if opt.sensor {
                    sensor_view.set_content(Sensor::stat(&amdgpu_dev));
                } else { 
                    sensor_view.set_content("");
                }

                if opt.high_freq {
                    sample = Sampling::high();
                } else {
                    sample = Sampling::low();
                }
            } else {
                cb_sink.send(Box::new(cursive::Cursive::quit)).unwrap();
                return;
            }

            grbm.clear();
            srbm.clear();
            srbm2.clear();
            cp_stat.clear();

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
    });

    siv.run();
}

fn check_register_offset(amdgpu_dev: &AMDGPU::DeviceHandle, name: &str, offset: u32) {
    if let Err(err) = amdgpu_dev.read_mm_registers(offset) {
        eprintln!("{name} ({offset:#X}) register could not be read. ({err})");
        dump_info(amdgpu_dev);
        panic!();
    }
}

fn dump_info(amdgpu_dev: &AMDGPU::DeviceHandle) {
    if let Ok(drm_ver) = amdgpu_dev.get_drm_version() {
        let (major, minor, patchlevel) = drm_ver;
        println!("drm version:\t{major}.{minor}.{patchlevel}");
    }

    if let Ok(mark_name) = amdgpu_dev.get_marketing_name() {
        println!("Marketing Name:\t[{mark_name}]");
    }

    if let Ok(ext_info) = amdgpu_dev.device_info() {
        println!(
            concat!(
                "DeviceID.RevID:\t{did:#04X}.{rid:#04X}\n",
                "Family:\t{family}\n",
                "ASIC:\t{asic}\n",
                "Chip class:\t{chip_class}\n",
                "VRAM: {vram_type} {vram_width}-bits"
            ),
            did = ext_info.device_id(),
            rid = ext_info.pci_rev_id(),
            family = ext_info.get_family_name(),
            asic = ext_info.get_asic_name(),
            chip_class = ext_info.get_chip_class(),
            vram_type = ext_info.get_vram_type(),
            vram_width = ext_info.vram_bit_width,
        );
    }
}

fn set_global_cb(siv: &mut cursive::Cursive) {
    type Opt = Arc<Mutex<UserOptions>>;

    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('u', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.uvd ^= true;
        });
    });
    siv.add_global_callback('s', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.srbm ^= true;
        });
    });
    siv.add_global_callback('g', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.grbm ^= true;
        });
    });
    siv.add_global_callback('c', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.cp_stat ^= true;
        });
    });
    siv.add_global_callback('v', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.vram ^= true;
        });
    });
    siv.add_global_callback('n', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.sensor ^= true;
        });
    });
    siv.add_global_callback('h', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.high_freq ^= true;
        });
    });
}
