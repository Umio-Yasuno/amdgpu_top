use cursive::views::{TextView, LinearLayout, Panel};
use cursive::view::Scrollable;
use cursive::align::HAlign;
use libdrm_amdgpu_sys::*;
use AMDGPU::{CHIP_CLASS, GPU_INFO};
use AMDGPU::{GRBM_OFFSET, GRBM2_OFFSET, SRBM_OFFSET, SRBM2_OFFSET, CP_STAT_OFFSET};
use std::sync::{Arc, Mutex};

mod grbm;
mod grbm2;
mod srbm;
mod srbm2;
mod cp_stat;
mod vram_usage;
mod args;
mod sensors;
mod gem_info;
mod pci;
mod util;

#[derive(Debug, Clone)]
struct ToggleOptions {
    grbm: bool,
    grbm2: bool,
    uvd: bool,
    srbm: bool,
    cp_stat: bool,
    pci: bool,
    vram: bool,
    sensor: bool,
    high_freq: bool,
    gem: bool,
}

impl Default for ToggleOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            grbm2: true,
            uvd: true,
            srbm: true,
            cp_stat: false,
            pci: true,
            vram: true,
            sensor: true,
            high_freq: false,
            gem: true,
        }
    }
}

type Opt = Arc<Mutex<ToggleOptions>>;

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

const TOGGLE_HELP: &str = " (g)rbm g(r)bm2 (u)vd (s)rbm (c)p_stat (p)ci\n (v)ram g(e)m se(n)sor (h)igh_freq (q)uit";

fn main() {
    let main_opt = args::MainOpt::parse();

    let (amdgpu_dev, major, minor) = {
        use std::fs::File;
        use std::os::fd::IntoRawFd;

        let path = format!("/dev/dri/renderD{}", 128 + main_opt.instance);
        let f = File::open(path).unwrap();

        AMDGPU::DeviceHandle::init(f.into_raw_fd()).unwrap()
    };
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let chip_class = ext_info.get_chip_class();

    let (min_gpu_clk, min_memory_clk) = util::get_min_clk(&amdgpu_dev, &pci_bus);
    let mark_name = match amdgpu_dev.get_marketing_name() {
        Ok(name) => name,
        Err(_) => "".to_string(), // unreachable
    };
    let info_bar = format!(
        concat!(
            "{mark_name} ({did:#06X}:{rid:#04X})\n",
            "{asic}, {chip_class}, {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
            "{vram_type} {vram_bus_width}-bit, {vram_size} MiB, ",
            "{min_memory_clk}-{max_memory_clk} MHz",
        ),
        mark_name = mark_name,
        did = ext_info.device_id(),
        rid = ext_info.pci_rev_id(),
        asic = ext_info.get_asic_name(),
        chip_class = chip_class,
        num_cu = ext_info.cu_active_number(),
        min_gpu_clk = min_gpu_clk,
        max_gpu_clk = ext_info.max_engine_clock().saturating_div(1000),
        vram_type = ext_info.get_vram_type(),
        vram_bus_width = ext_info.vram_bit_width,
        vram_size = memory_info.vram.total_heap_size >> 20,
        min_memory_clk = min_memory_clk,
        max_memory_clk = ext_info.max_memory_clock().saturating_div(1000),
    );

    if main_opt.dump {
        let link = pci_bus.get_link_info(PCI::STATUS::Current);

        println!("--- AMDGPU info dump ---");
        println!("drm: {major}.{minor}");
        println!("{info_bar}");
        println!("PCI (domain:bus:dev.func): {pci_bus}");
        println!("PCI Link: Gen{}x{}", link.gen, link.width);
        util::vbios_info(&amdgpu_dev);
        return;
    }

    let gem_info_path = format!(
        "/sys/kernel/debug/dri/{i}/amdgpu_gem_info",
        i = main_opt.instance,
    );

    let mut grbm = grbm::GRBM::default();
    let mut grbm2 = grbm2::GRBM2::default();
    let mut uvd = srbm::SRBM::default();
    let mut srbm2 = srbm2::SRBM2::default();
    let mut cp_stat = cp_stat::CP_STAT::default();
    let mut vram = vram_usage::VRAM_INFO::new(&memory_info);
    let mut gem_info = gem_info::GemView::default();
    let mut sensor = sensors::Sensor::default();
    let mut pci = pci::PCI_LINK_INFO::new(&pci_bus);

    let mut toggle_opt = ToggleOptions::default();

    {   // check register offset
        [toggle_opt.grbm, grbm.flag] =
            [util::check_register_offset(&amdgpu_dev, "mmGRBM_STATUS", GRBM_OFFSET); 2];
        [toggle_opt.grbm2, grbm2.flag] =
            [util::check_register_offset(&amdgpu_dev, "mmGRBM2_STATUS", GRBM2_OFFSET); 2];

        [toggle_opt.uvd, uvd.flag] =
            [util::check_register_offset(&amdgpu_dev, "mmSRBM_STATUS", SRBM_OFFSET); 2];

        [toggle_opt.srbm, srbm2.flag] =
            [util::check_register_offset(&amdgpu_dev, "mmSRBM_STATUS2", SRBM2_OFFSET); 2];

        let _ = util::check_register_offset(&amdgpu_dev, "mmCP_STAT", CP_STAT_OFFSET);
        [toggle_opt.cp_stat, cp_stat.flag] = [false; 2];

        grbm.is_gfx10_plus = CHIP_CLASS::GFX10 <= chip_class;

        if let Ok(ref mut f) = std::fs::File::open(&gem_info_path) {
            toggle_opt.gem = true;

            gem_info.read_to_print(f);
            gem_info.text.set();
        } else {
            toggle_opt.gem = false;
        }

        // fill
        grbm.dump();
        grbm2.dump();
        uvd.dump();
        srbm2.dump();
        cp_stat.dump();
        {
            vram.print();
            vram.text.set();
        }
        {
            sensor.print(&amdgpu_dev);
            sensor.text.set();
        }
        {
            pci.print();
            pci.text.set();
        }
    }

    let mut siv = cursive::default();
    {
        let mut layout = LinearLayout::vertical()
            .child(
                Panel::new(
                    TextView::new(&info_bar).center()
                )
                .title("amdgpu_top")
                .title_position(HAlign::Center)
            )
            .child(grbm.text.panel("GRBM"));

        if toggle_opt.grbm2 {
            layout.add_child(grbm2.text.panel("GRBM2"));
            siv.add_global_callback('r', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.grbm2 ^= true;
                });
            });
        }
        // mmSRBM_STATUS/mmSRBM_STATUS2 does not exist in GFX9 (soc15) or later.
        if toggle_opt.uvd && (chip_class < CHIP_CLASS::GFX9) {
            layout.add_child(uvd.text.panel("UVD"));
            siv.add_global_callback('u', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.uvd ^= true;
                });
            });
        }
        if toggle_opt.srbm && (chip_class < CHIP_CLASS::GFX9) {
            layout.add_child(srbm2.text.panel("SRBM2"));
            siv.add_global_callback('s', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.srbm ^= true;
                });
            });
        }

        layout.add_child(cp_stat.text.panel("CP_STAT"));
        layout.add_child(pci.text.panel("PCI"));
        layout.add_child(vram.text.panel("Memory Usage"));

        if toggle_opt.gem {
            layout.add_child(gem_info.text.panel("GEM Info"));
            siv.add_global_callback('e', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.gem ^= true;
                });
            });
        }

        layout.add_child(sensor.text.panel("Sensors"));
        layout.add_child(TextView::new(TOGGLE_HELP));

        siv.add_layer(
            layout
                .scrollable()
                .scroll_y(true)
        );
    }

    let toggle_opt = Arc::new(Mutex::new(toggle_opt));
    siv.set_user_data(toggle_opt.clone());
    set_global_cb(&mut siv);

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || {
        let mut sample = Sampling::low();
        let opt = toggle_opt;

        loop {
            for _ in 0..sample.count {
                // high frequency accesses to registers can cause high GPU clocks
                if grbm.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(GRBM_OFFSET) {
                        grbm.bits.acc(out);
                    }
                }
                if grbm2.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(GRBM2_OFFSET) {
                        grbm2.bits.acc(out);
                    }
                }
                if uvd.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(SRBM_OFFSET) {
                        uvd.bits.acc(out);
                    }
                }
                if srbm2.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(SRBM2_OFFSET) {
                        srbm2.bits.acc(out);
                    }
                }
                if cp_stat.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(CP_STAT_OFFSET) {
                        cp_stat.bits.acc(out);
                    }
                }

                std::thread::sleep(sample.delay);
            }

            if let Ok(opt) = opt.try_lock() {
                grbm.flag = opt.grbm;
                grbm2.flag = opt.grbm2;
                uvd.flag = opt.uvd;
                srbm2.flag = opt.srbm;
                cp_stat.flag = opt.cp_stat;

                if opt.pci {
                    pci.update_status();
                    pci.print();
                } else {
                    pci.text.clear();
                }

                if opt.vram {
                    vram.update_usage(&amdgpu_dev);
                    vram.print();
                } else {
                    vram.text.clear();
                }

                if opt.sensor {
                    sensor.print(&amdgpu_dev);
                } else { 
                    sensor.text.clear();
                }

                if opt.gem {
                    if let Ok(ref mut f) = std::fs::File::open(&gem_info_path) {
                        gem_info.read_to_print(f);
                    }
                } else {
                    gem_info.clear();
                }

                sample = if opt.high_freq {
                    Sampling::high()
                } else {
                    Sampling::low()
                };
            }

            grbm.dump();
            grbm2.dump();
            uvd.dump();
            cp_stat.dump();
            srbm2.dump();

            vram.text.set();
            pci.text.set();
            gem_info.text.set();
            sensor.text.set();

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
    });

    siv.run();
}

fn set_global_cb(siv: &mut cursive::Cursive) {
    siv.add_global_callback('q', cursive::Cursive::quit);
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
    siv.add_global_callback('p', |s| {
        s.with_user_data(|opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.pci ^= true;
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
