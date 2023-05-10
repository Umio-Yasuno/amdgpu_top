use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::view::{Nameable, Scrollable};
use cursive::{event::Key, menu, traits::With};

use libamdgpu_top::{stat, DevicePath, Sampling};
use stat::ProcInfo;

mod view;
use view::*;

mod app;
use app::TuiApp;

#[derive(Debug, Clone)]
struct ToggleOptions {
    grbm: bool,
    grbm2: bool,
    vram: bool,
    sensor: bool,
    high_freq: bool,
    fdinfo: bool,
    fdinfo_sort: stat::FdInfoSortType,
    reverse_sort: bool,
    gpu_metrics: bool,
}

impl Default for ToggleOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            grbm2: true,
            vram: true,
            sensor: true,
            high_freq: false,
            fdinfo: true,
            fdinfo_sort: Default::default(),
            reverse_sort: false,
            gpu_metrics: false,
        }
    }
}

type Opt = Arc<Mutex<ToggleOptions>>;

/*
pub const TOGGLE_HELP: &str = concat!(
    " (g)rbm g(r)bm2 (v)ram_usage (f)dinfo \n",
    " se(n)sor (m)etrics (h)igh_freq (q)uit \n",
    " (P): sort_by_pid (V): sort_by_vram (G): sort_by_gfx\n (M): sort_by_media (R): reverse"
);
*/
pub const TOGGLE_HELP: &str = concat!(
    " (f)dinfo se(n)sor (m)etrics (h)igh_freq (q)uit \n",
    " (P): sort_by_pid (V): sort_by_vram (G): sort_by_gfx\n (M): sort_by_media (R): reverse"
);

pub fn run(
    title: &str,
    device_path_list: &[DevicePath],
    interval: u64,
) {
    let mut toggle_opt = ToggleOptions::default();

    let mut vec_app: Vec<TuiApp> = device_path_list.iter().map(|device_path| {
        let amdgpu_dev = device_path.init().unwrap();
        let ext_info = amdgpu_dev.device_info().unwrap();
        let memory_info = amdgpu_dev.memory_info().unwrap();

        let mut app = app::TuiApp::new(amdgpu_dev, &device_path, &ext_info, &memory_info);
        app.fill(&mut toggle_opt);

        app
    }).collect();

    let mut siv = cursive::default();

    {
        // TODO: update for multi-layers
        // siv.add_global_callback('g', pc_type_cb(&PCType::GRBM));
        // siv.add_global_callback('r', pc_type_cb(&PCType::GRBM2));
        // siv.add_global_callback('v', VramUsageView::cb);
        siv.add_global_callback('f', FdInfoView::cb);
        siv.add_global_callback('R', FdInfoView::cb_reverse_sort);
        siv.add_global_callback('P', FdInfoView::cb_sort_by_pid);
        siv.add_global_callback('V', FdInfoView::cb_sort_by_vram);
        siv.add_global_callback('G', FdInfoView::cb_sort_by_gfx);
        siv.add_global_callback('M', FdInfoView::cb_sort_by_media);
        siv.add_global_callback('n', SensorsView::cb);
        siv.add_global_callback('m', GpuMetricsView::cb);
        siv.add_global_callback('q', cursive::Cursive::quit);
        siv.add_global_callback('h', |siv| {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.high_freq ^= true;
        });
        siv.add_global_callback(Key::Esc, |siv| siv.select_menubar());
    }
    {
        let menubar = siv.menubar();
        
        menubar.add_subtree(
            "Device List [ESC]",
            menu::Tree::new()
                .with(|tree| { for app in &vec_app {
                    let name = app.list_name.clone();
                    tree.add_leaf(
                        name.clone(),
                        move |siv: &mut cursive::Cursive| {
                            let screen = siv.screen_mut();
                            let Some(pos) = screen.find_layer_from_name(&name) else { return };
                            screen.move_to_front(pos);
                        },
                    );
                }})
                .delimiter()
                .leaf("Quit", cursive::Cursive::quit),
        );
    }
    {
        let screen = siv.screen_mut();
        for app in &vec_app {
            screen.add_layer(
                app.layout(title, &toggle_opt)
                    .scrollable()
                    .scroll_y(true)
                    .with_name(&app.list_name)
            );
        }
    }

    for app in &vec_app {
        if app.support_pcie_bw {
            if let Ok(pcie_bw) = app.arc_pcie_bw.lock() {
                let arc_pcie_bw = app.arc_pcie_bw.clone();
                let mut buf_pcie_bw = pcie_bw.clone();

                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_millis(500)); // wait for user input
                    buf_pcie_bw.update(); // msleep(1000)

                    let lock = arc_pcie_bw.lock();
                    if let Ok(mut pcie_bw) = lock {
                        *pcie_bw = buf_pcie_bw.clone();
                    }
                });
            }
        }
    }
    {
        let t_index: Vec<(DevicePath, Arc<Mutex<Vec<ProcInfo>>>)> = vec_app.iter().map(|app| {
            (app.device_path.clone(), app.arc_proc_index.clone())
        }).collect();
        let mut buf_index: Vec<ProcInfo> = Vec::new();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(interval));

            for (device_path, index) in &t_index {
                stat::update_index(&mut buf_index, &device_path);

                let lock = index.lock();
                if let Ok(mut index) = lock {
                    *index = buf_index.clone();
                }
            }
        });
    }

    let mut flags = toggle_opt.clone();
    let toggle_opt = Arc::new(Mutex::new(toggle_opt));

    siv.set_autohide_menu(false);
    siv.set_user_data(toggle_opt.clone());

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || loop {
        {
            let lock = toggle_opt.try_lock();
            if let Ok(opt) = lock {
                flags = opt.clone();
            }
        }

        let sample = if flags.high_freq { Sampling::high() } else { Sampling::low() };

        for _ in 0..sample.count {
            for app in vec_app.iter_mut() {
                app.update_pc(&flags);
            }

            std::thread::sleep(sample.delay);
        }

        for app in vec_app.iter_mut() {
            app.update(&flags, &sample);
        }

        cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
    });

    siv.run();
}
