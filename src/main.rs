use cursive::views::{Dialog, TextContent, TextView, ProgressBar, LinearLayout};
use libdrm_amdgpu_sys::*;
use AMDGPU::GPU_INFO;

mod grbm;
use grbm::*;

mod srbm;
use srbm::*;

mod cp_stat;
use cp_stat::*;

mod vram_usage;
use vram_usage::*;

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
    let mut cp_stat = CP_STAT::new();

    let grbm_view = TextContent::new(grbm.verbose_stat()); // 0%
    let srbm_view = TextContent::new(srbm.stat()); // 0%
    let cp_stat_view = TextContent::new(cp_stat.stat());
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
    siv.add_layer(
        LinearLayout::vertical()
            .child(TextView::new(format!("WIP: amdgpu_top")).center())
            .child(TextView::new(
                if let Ok(mark_name) = amdgpu_dev.get_marketing_name() {
                    mark_name
                } else {
                    "".to_string()
                }
            ).center())
            .child(TextView::new(info_bar).center())
            .child(TextView::new_with_content(grbm_view.clone()).center())
            .child(TextView::new_with_content(srbm_view.clone()).center())
            .child(TextView::new_with_content(cp_stat_view.clone()).center())
            .child(TextView::new_with_content(vram_view.clone()).center())
            .child(TextView::new("\n___").center())
    );
    siv.add_global_callback('q', cursive::Cursive::quit);
    /*
    siv.add_global_callback('u', |s| {
        s.call_on(
            &view::Selector::Name("Debug1"),
            |view: &mut TextView| {
                view.set_content("UUU");
            },
        );
    });
    */
    let cb_sink = siv.cb_sink().clone();
    let delay = std::time::Duration::from_millis(1);

    let grbm_offset = ext_info.get_family_name().get_grbm_offset();
    let srbm_offset = ext_info.get_family_name().get_srbm_offset();
    let cp_stat_offset = ext_info.get_family_name().get_cp_stat_offset();

    std::thread::spawn(move || {
        loop {
            for _ in 0..100 {
                if let Ok(out) = amdgpu_dev.read_mm_registers(grbm_offset) {
                    grbm.acc(out);
                }
                if let Ok(out) = amdgpu_dev.read_mm_registers(srbm_offset) {
                    srbm.acc(out);
                }
                if let Ok(out) = amdgpu_dev.read_mm_registers(cp_stat_offset) {
                    cp_stat.acc(out);
                }
                std::thread::sleep(delay);
            }

            grbm_view.set_content(grbm.verbose_stat());
            srbm_view.set_content(srbm.stat());
            cp_stat_view.set_content(cp_stat.stat());

            grbm.clear();
            srbm.clear();
            cp_stat.clear();

            if let Ok(info) = amdgpu_dev.memory_info() {
                vram_view.set_content(VRAM_USAGE::from(info).stat());
            }

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
        cb_sink.send(Box::new(cursive::Cursive::quit)).unwrap();
    });

    // Starts the event loop.
    siv.run();
}
