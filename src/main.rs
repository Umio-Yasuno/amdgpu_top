use cursive::views::{TextContent, TextView, LinearLayout, Panel};
use cursive::align::HAlign;
use libdrm_amdgpu_sys::*;
use AMDGPU::GPU_INFO;
use std::sync::{Arc, Mutex};

mod grbm;
mod srbm;
mod srbm2;
mod cp_stat;
mod vram_usage;
mod args;
mod sensors;
mod gem_info;

#[derive(Debug, Clone)]
struct ToggleOptions {
    grbm: bool,
    uvd: bool,
    srbm: bool,
    cp_stat: bool,
    vram: bool,
    sensor: bool,
    high_freq: bool,
    gem: bool,
}

impl Default for ToggleOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            uvd: true,
            srbm: true,
            cp_stat: false,
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

const TOGGLE_HELP: &str = " v(g)t (u)vd (s)rbm (c)p_stat\n (v)ram g(e)m se(n)sor (h)igh_freq (q)uit";

fn main() {
    let main_opt = args::MainOpt::parse();

    let (amdgpu_dev, _major, _minor) = {
        use std::fs::File;
        use std::os::fd::IntoRawFd;

        let path = format!("/dev/dri/renderD{}", 128 + main_opt.instance);
        let f = File::open(&path).unwrap();

        AMDGPU::DeviceHandle::init(f.into_raw_fd()).unwrap()
    };
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let (min_gpu_clk, min_memory_clk) = get_min_clk(&amdgpu_dev);
    let mark_name = match amdgpu_dev.get_marketing_name() {
        Ok(name) => name,
        Err(_) => "".to_string(),
    };
    let info_bar = format!(
        concat!(
            "{mark_name} ({did:#06X}:{rid:#04X})\n",
            "{asic}, {num_cu} CU, {min_gpu_clk}-{max_gpu_clk} MHz\n",
            "{vram_type} {vram_bus_width}-bit, {vram_size} MiB, ",
            "{min_memory_clk}-{max_memory_clk} MHz",
        ),
        mark_name = mark_name,
        did = ext_info.device_id(),
        rid = ext_info.pci_rev_id(),
        asic = ext_info.get_asic_name(),
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
        println!("--- AMDGPU info dump ---\n{info_bar}");
        return;
    }

    let gem_info_path = format!(
        "/sys/kernel/debug/dri/{i}/amdgpu_gem_info",
        i = main_opt.instance,
    );

    let mut grbm = grbm::GRBM::new();
    let mut uvd = srbm::SRBM::new();
    let mut srbm2 = srbm2::SRBM2::new();
    let mut cp_stat = cp_stat::CP_STAT::new();
    let mut vram = vram_usage::VRAM_INFO::from(&memory_info);
    let mut gem = gem_info::GemView::default();

    let grbm_offset = AMDGPU::GRBM_OFFSET;
    let srbm_offset = AMDGPU::SRBM_OFFSET;
    let srbm2_offset = AMDGPU::SRBM2_OFFSET;
    let cp_stat_offset = AMDGPU::CP_STAT_OFFSET;

    let mut toggle_opt = ToggleOptions::default();

    {   // check register offset
        toggle_opt.grbm = check_register_offset(&amdgpu_dev, "mmGRBM_STATUS", grbm_offset);
        grbm.flag = toggle_opt.grbm;

        toggle_opt.uvd = check_register_offset(&amdgpu_dev, "mmSRBM_STATUS", srbm_offset);
        uvd.flag = toggle_opt.uvd;

        toggle_opt.srbm = check_register_offset(&amdgpu_dev, "mmSRBM_STATUS2", srbm2_offset);
        srbm2.flag = toggle_opt.srbm;

        let _ = check_register_offset(&amdgpu_dev, "mmCP_STAT", cp_stat_offset);
        toggle_opt.cp_stat = false;
        cp_stat.flag = false;

        if let Ok(ref mut f) = std::fs::File::open(&gem_info_path) {
            toggle_opt.gem = true;

            gem.set(f);
        } else {
            toggle_opt.gem = false;
        }
    }

    let grbm_view = TextContent::new(grbm.stat());
    let uvd_view = TextContent::new(uvd.stat());
    let srbm2_view = TextContent::new(srbm2.stat());
    let cp_stat_view = TextContent::new(cp_stat.stat());
    let sensor_view = TextContent::new(sensors::Sensor::stat(&amdgpu_dev));
    let vram_view = TextContent::new(vram.stat());
    let gem_info_view = TextContent::new(&gem.buf);

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
            .child(
                Panel::new(
                    TextView::new_with_content(grbm_view.clone())
                )
                .title("GRBM")
                .title_position(HAlign::Left)
            );
        // mmSRBM_STATUS/mmSRBM_STATUS2 does not exist in GFX9 (soc15) or later.
        if toggle_opt.uvd {
            layout.add_child(
                Panel::new(
                    TextView::new_with_content(uvd_view.clone())
                )
                .title("UVD")
                .title_position(HAlign::Left)
            );
            siv.add_global_callback('u', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.uvd ^= true;
                });
            });
        }
        if toggle_opt.srbm {
            layout.add_child(
                Panel::new(
                    TextView::new_with_content(srbm2_view.clone())
                )
                .title("SRBM2")
                .title_position(HAlign::Left)
            );
            siv.add_global_callback('s', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.srbm ^= true;
                });
            });
        }
        layout.add_child(
            Panel::new(
                TextView::new_with_content(cp_stat_view.clone())
            )
            .title("CP_STAT")
            .title_position(HAlign::Left)
        );
        layout.add_child(
            Panel::new(
                TextView::new_with_content(vram_view.clone())
            )
            .title("Memory Usage")
            .title_position(HAlign::Left)
        );
        if toggle_opt.gem {
            layout.add_child(
                Panel::new(
                    TextView::new_with_content(gem_info_view.clone())
                )
                .title("GEM Info")
                .title_position(HAlign::Left)
            );
            siv.add_global_callback('e', |s| {
                s.with_user_data(|opt: &mut Opt| {
                    let mut opt = opt.lock().unwrap();
                    opt.gem ^= true;
                });
            });
        }
        layout.add_child(
            Panel::new(
                TextView::new_with_content(sensor_view.clone())
            )
            .title("Sensors")
            .title_position(HAlign::Left)
        );
        layout.add_child(TextView::new(TOGGLE_HELP));

        siv.add_layer(layout);
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
                    if let Ok(out) = amdgpu_dev.read_mm_registers(grbm_offset) {
                        grbm.acc(out);
                    }
                }
                if uvd.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(srbm_offset) {
                        uvd.acc(out);
                    }
                }
                if srbm2.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(srbm2_offset) {
                        srbm2.acc(out);
                    }
                }
                if cp_stat.flag {
                    if let Ok(out) = amdgpu_dev.read_mm_registers(cp_stat_offset) {
                        cp_stat.acc(out);
                    }
                }

                std::thread::sleep(sample.delay);
            }

            if let Ok(opt) = opt.try_lock() {
                grbm.flag = opt.grbm;
                uvd.flag = opt.uvd;
                srbm2.flag = opt.srbm;
                cp_stat.flag = opt.cp_stat;

                if opt.vram {
                    if let [Ok(usage_vram), Ok(usage_gtt)] = [
                        amdgpu_dev.vram_usage_info(),
                        amdgpu_dev.gtt_usage_info(),
                    ] {
                        vram.usage_vram = usage_vram;
                        vram.usage_gtt = usage_gtt;

                        vram_view.set_content(vram.stat());
                    }
                } else {
                    vram_view.set_content("");
                }

                if opt.sensor {
                    sensor_view.set_content(sensors::Sensor::stat(&amdgpu_dev));
                } else { 
                    sensor_view.set_content("");
                }

                if opt.gem {
                    if let Ok(ref mut f) = std::fs::File::open(&gem_info_path) {
                        gem.set(f);
                    }
                } else {
                    gem.clear();
                }

                sample = if opt.high_freq {
                    Sampling::high()
                } else {
                    Sampling::low()
                };
            }

            grbm_view.set_content(grbm.stat());
            uvd_view.set_content(uvd.stat());
            srbm2_view.set_content(srbm2.stat());
            cp_stat_view.set_content(cp_stat.stat());
            gem_info_view.set_content(&gem.buf);

            grbm.clear();
            uvd.clear();
            srbm2.clear();
            cp_stat.clear();

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
    });

    siv.run();
}

fn get_min_clk(amdgpu_dev: &AMDGPU::DeviceHandle) -> (u64, u64) {
    if let Ok(pci_bus) = amdgpu_dev.get_pci_bus_info() {
        if let [Some(gpu), Some(mem)] = [
            amdgpu_dev.get_min_gpu_clock_from_sysfs(&pci_bus),
            amdgpu_dev.get_min_memory_clock_from_sysfs(&pci_bus),
        ] {
            (gpu, mem)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    }
}

fn check_register_offset(amdgpu_dev: &AMDGPU::DeviceHandle, name: &str, offset: u32) -> bool {
    if let Err(err) = amdgpu_dev.read_mm_registers(offset) {
        println!("{name} ({offset:#X}) register is not allowed. ({err})");
        return false;
    }

    true
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
