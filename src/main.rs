use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::{DeviceHandle, CHIP_CLASS, GPU_INFO}
};
use std::sync::{Arc, Mutex};
use cursive::views::{TextView, LinearLayout, Panel};
use cursive::view::Scrollable;
use cursive::align::HAlign;

mod stat;
mod args;
mod misc;

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
            cp_stat: true,
            pci: true,
            vram: true,
            sensor: true,
            high_freq: false,
            gem: true,
        }
    }
}

type Opt = Arc<Mutex<ToggleOptions>>;

const TOGGLE_HELP: &str = concat!(
    " (g)rbm g(r)bm2 (u)vd (s)rbm (c)p_stat (p)ci\n",
    " (v)ram g(e)m se(n)sor (h)igh_freq (q)uit",
);

fn main() {
    let main_opt = args::MainOpt::parse();

    let (amdgpu_dev, major, minor) = {
        use std::fs::File;
        use std::os::fd::IntoRawFd;

        let path = format!("/dev/dri/renderD{}", 128 + main_opt.instance);
        let f = File::open(path).unwrap();

        DeviceHandle::init(f.into_raw_fd()).unwrap()
    };
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let chip_class = ext_info.get_chip_class();

    let (min_gpu_clk, min_memory_clk) = misc::get_min_clk(&amdgpu_dev, &pci_bus);
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
        misc::vbios_info(&amdgpu_dev);
        return;
    }

    let gem_info_path = format!(
        "/sys/kernel/debug/dri/{i}/amdgpu_gem_info",
        i = main_opt.instance,
    );

    let mut grbm = stat::GRBM::new(CHIP_CLASS::GFX10 <= chip_class);
    let mut grbm2 = stat::GRBM2::new();
    let mut uvd = stat::SRBM::new();
    let mut srbm2 = stat::SRBM2::new();
    let mut cp_stat = stat::CP_STAT::new();
    let mut vram = stat::VRAM_INFO::new(&memory_info);
    let mut gem_info = stat::GemView::default();
    let mut sensor = stat::Sensor::default();
    let mut pci = stat::PCI_LINK_INFO::new(&pci_bus);

    let mut toggle_opt = ToggleOptions::default();

    {   // check register offset
        toggle_opt.grbm = stat::GRBM::check_reg_offset(&amdgpu_dev);
        toggle_opt.grbm2 = stat::GRBM2::check_reg_offset(&amdgpu_dev);
        toggle_opt.uvd = stat::SRBM::check_reg_offset(&amdgpu_dev);
        toggle_opt.srbm = stat::SRBM2::check_reg_offset(&amdgpu_dev);
        [toggle_opt.cp_stat, _] = [false, stat::CP_STAT::check_reg_offset(&amdgpu_dev)];

        if let Ok(ref mut f) = std::fs::File::open(&gem_info_path) {
            toggle_opt.gem = true;

            gem_info.read_to_print(f);
            gem_info.text.set();
        } else {
            toggle_opt.gem = false;
        }

        // fill
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
            );

        if toggle_opt.grbm {
            layout.add_child(grbm.top_view());
            siv.add_global_callback('g', stat::GRBM::cb);
        }
        if toggle_opt.grbm2 {
            layout.add_child(grbm2.top_view());
            siv.add_global_callback('r', stat::GRBM2::cb);
        }
        // mmSRBM_STATUS/mmSRBM_STATUS2 does not exist in GFX9 (soc15) or later.
        if toggle_opt.uvd && (chip_class < CHIP_CLASS::GFX9) {
            layout.add_child(uvd.top_view());
            siv.add_global_callback('u', stat::SRBM::cb);
        }
        if toggle_opt.srbm && (chip_class < CHIP_CLASS::GFX9) {
            layout.add_child(srbm2.top_view());
            siv.add_global_callback('s', stat::SRBM2::cb);
        }
        {
            let visible = toggle_opt.cp_stat;
            layout.add_child(cp_stat.top_view(visible));
            siv.add_global_callback('c', stat::CP_STAT::cb);
        }
        {
            layout.add_child(pci.text.panel("PCI"));
            siv.add_global_callback('p', stat::PCI_LINK_INFO::cb);
        }
        {
            layout.add_child(vram.text.panel("Memory Usage"));
            siv.add_global_callback('v', stat::VRAM_INFO::cb);
        }
        if toggle_opt.gem {
            layout.add_child(gem_info.text.panel("GEM Info"));
            siv.add_global_callback('e', stat::GemView::cb);
        }
        {
            layout.add_child(sensor.text.panel("Sensors"));
            siv.add_global_callback('n', stat::Sensor::cb);
        }
        layout.add_child(TextView::new(TOGGLE_HELP));

        siv.add_layer(
            layout
                .scrollable()
                .scroll_y(true)
        );
    }

    let mut flags = toggle_opt.clone();
    let toggle_opt = Arc::new(Mutex::new(toggle_opt));
    siv.set_user_data(toggle_opt.clone());
    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('h', Sampling::cb);

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || {
        let mut sample = Sampling::low();

        loop {
            if let Ok(opt) = toggle_opt.try_lock() {
                flags = opt.clone();
            }

            for _ in 0..sample.count {
                // high frequency accesses to registers can cause high GPU clocks
                if flags.grbm {
                    grbm.read_reg(&amdgpu_dev);
                }
                if flags.grbm2 {
                    grbm2.read_reg(&amdgpu_dev);
                }
                if flags.uvd {
                    uvd.read_reg(&amdgpu_dev);
                }
                if flags.srbm {
                    srbm2.read_reg(&amdgpu_dev);
                }
                if flags.cp_stat {
                    cp_stat.read_reg(&amdgpu_dev);
                }

                std::thread::sleep(sample.delay);
            }

            if flags.pci {
                pci.update_status();
                pci.print();
            } else {
                pci.text.clear();
            }

            if flags.vram {
                vram.update_usage(&amdgpu_dev);
                vram.print();
            } else {
                vram.text.clear();
            }

            if flags.sensor {
                sensor.print(&amdgpu_dev);
            } else {
                sensor.text.clear();
            }

            if flags.gem {
                if let Ok(ref mut f) = std::fs::File::open(&gem_info_path) {
                    gem_info.read_to_print(f);
                }
            } else {
                gem_info.clear();
            }

            sample = if flags.high_freq {
                Sampling::high()
            } else {
                Sampling::low()
            };

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

use std::time::Duration;

struct Sampling {
    count: usize,
    delay: Duration,
}

impl Sampling {
    const fn low() -> Self {
        Self {
            count: 100,
            delay: Duration::from_millis(10),
        }
    }

    const fn high() -> Self {
        Self {
            count: 100,
            delay: Duration::from_millis(1),
        }
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.high_freq ^= true;
        }
    }
}
