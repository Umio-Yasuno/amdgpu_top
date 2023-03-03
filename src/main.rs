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
        }
    }
}

type Opt = Arc<Mutex<UserOptions>>;

const TOGGLE_HELP: &str = "\n v(g)t (u)vd (s)rbm (c)p_stat\n (v)ram se(n)sor (q)uit";

fn main() {
    let (amdgpu_dev, _major, _minor) = {
        use std::fs::File;
        use std::os::fd::IntoRawFd;

        let fd = File::open("/dev/dri/renderD128").unwrap();

        AMDGPU::DeviceHandle::init(fd.into_raw_fd()).unwrap()
    };
    let ext_info = amdgpu_dev.device_info().unwrap();
    let family_name = ext_info.get_family_name();

    let mut grbm = GRBM::new();
    let mut srbm = SRBM::new();
    let mut srbm2 = SRBM2::new();
    let mut cp_stat = CP_STAT::new();

    let grbm_offset = family_name.get_grbm_offset();
    let srbm_offset = family_name.get_srbm_offset();
    let srbm2_offset = family_name.get_srbm2_offset();
    let cp_stat_offset = family_name.get_cp_stat_offset();

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

    let info_bar = format!(
        "{asic}, {num_cu} CU, {vram_type} {vram_bus_width}-bit",
        asic = ext_info.get_asic_name(),
        num_cu = ext_info.cu_active_number(),
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
    );

    let mut siv = cursive::default();
    let user_opt = Arc::new(Mutex::new(UserOptions::default()));
    let mark_name = match amdgpu_dev.get_marketing_name() {
        Ok(name) => name,
        Err(_) => "".to_string(),
    };

    siv.add_layer(
        LinearLayout::vertical()
            .child(TextView::new("amdgpu_top").center())
            .child(TextView::new(mark_name).center())
            .child(TextView::new(info_bar).center())
            .child(TextView::new("\n"))
            .child(TextView::new_with_content(grbm_view.clone()).center())
            .child(TextView::new("\n"))
            .child(TextView::new_with_content(srbm_view.clone()).center())
            .child(TextView::new_with_content(srbm2_view.clone()).center())
            .child(TextView::new("\n"))
            .child(TextView::new_with_content(cp_stat_view.clone()).center())
            .child(TextView::new("\n"))
            .child(TextView::new_with_content(vram_view.clone()).center())
            .child(TextView::new("\n"))
            .child(TextView::new_with_content(sensor_view.clone()).center())
            .child(TextView::new("\n"))
            .child(TextView::new(TOGGLE_HELP))
    );
    set_global_cb(&mut siv);
    siv.set_user_data(user_opt.clone());

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || {
        let delay = std::time::Duration::from_millis(1);
        let opt = user_opt.clone();

        loop {
            for _ in 0..100 {
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
                std::thread::sleep(delay);
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
            "DeviceID.RevID:\t{:#0X}.{:#0X}",
            ext_info.device_id(),
            ext_info.pci_rev_id()
        );

        println!("Family:\t\t{}", ext_info.get_family_name());
        println!("ASIC Name:\t{}", ext_info.get_asic_name());
        println!("Chip class:\t{}", ext_info.get_chip_class());
        println!("VRAM Type:\t{}", ext_info.get_vram_type());
        println!("VRAM Bit Width:\t{}-bit", ext_info.vram_bit_width);
    }
}

fn set_global_cb(siv: &mut cursive::Cursive) {
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
}
