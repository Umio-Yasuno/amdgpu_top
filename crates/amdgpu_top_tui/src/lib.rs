use std::sync::{Arc, Mutex};
use cursive::view::{Nameable, Scrollable};
use cursive::{event::Key, menu, traits::With};
use cursive::theme::{BorderStyle, Theme, Palette};

use libamdgpu_top::{app::AppAmdgpuTop, DevicePath, Sampling};
use libamdgpu_top::stat::{self, FdInfoSortType, PCType};

mod view;
use view::*;

mod app;
use app::*;

mod smi;
pub use smi::run_smi;

#[derive(Debug, Clone)]
struct ToggleOptions {
    grbm: bool,
    grbm2: bool,
    vram: bool,
    activity: bool,
    sensor: bool,
    high_freq: bool,
    fdinfo: bool,
    fdinfo_sort: FdInfoSortType,
    reverse_sort: bool,
    gpu_metrics: bool,
    select_index: usize,
    indexes: Vec<usize>,
    is_dark_mode: bool,
}

impl Default for ToggleOptions {
    fn default() -> Self {
        Self {
            grbm: true,
            grbm2: true,
            vram: true,
            activity: true,
            sensor: true,
            high_freq: false,
            fdinfo: true,
            fdinfo_sort: Default::default(),
            reverse_sort: false,
            gpu_metrics: true,
            select_index: 0,
            indexes: Vec::new(),
            is_dark_mode: false,
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
    " (g)rbm g(r)bm2 (v)ram_usage (f)dinfo\n se(n)sor (m)etrics (h)igh_freq (q)uit \n",
    " (P): sort_by_pid (V): sort_by_vram (G): sort_by_gfx\n (M): sort_by_media (R): reverse \n",
    " (T): switch theme (light/dark)",
);

pub fn run(
    title: &str,
    selected_device_path: DevicePath,
    device_path_list: &[DevicePath],
    interval: u64,
    no_pc: bool,
    is_dark_mode: bool,
) {
    let title = title.to_string();
    let mut toggle_opt = ToggleOptions { is_dark_mode, ..Default::default() };

    let (vec_app, suspended_devices) = AppAmdgpuTop::create_app_and_suspended_list(
        device_path_list,
        &Default::default(),
    );
    let mut vec_app: Vec<_> = vec_app
        .into_iter()
        .enumerate()
        .map(|(i, app)| TuiApp::new_with_app(app, no_pc, i))
        .collect();
    let app_len = vec_app.len();
    let mut vec_sus_app: Vec<_> = suspended_devices
        .into_iter()
        .enumerate()
        .map(|(i, app)| SuspendedTuiApp::new(app, no_pc, app_len+i))
        .collect();

    for app in vec_app.iter_mut() {
        app.update(&toggle_opt, &Sampling::low());
    }

    toggle_opt.indexes = vec_app.iter().map(|app| app.index).collect();

    {
        let mut device_paths: Vec<DevicePath> = device_path_list.to_vec();

        if let Some(xdna_device_path) = vec_app
            .iter()
            .find_map(|app| app.app_amdgpu_top.xdna_device_path.as_ref())
        {
            device_paths.push(xdna_device_path.clone());
        }

        stat::spawn_update_index_thread(device_paths, interval);
    }

    let mut siv = cursive::default();
    {
        let menubar = siv.menubar();
        
        menubar.add_subtree(
            "Device List [ESC]",
            menu::Tree::new()
                .with(|tree| {
                    for app in &vec_app {
                        let index = app.index;

                        tree.add_leaf(
                            app.label(),
                            move |siv: &mut cursive::Cursive| {
                                let screen = siv.screen_mut();
                                let Some(pos) = screen.find_layer_from_name(&index.to_string())
                                    else { return };
                                screen.move_to_front(pos);

                                let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
                                opt.select_index = index;
                            },
                        );
                    }

                    for app in &vec_sus_app {
                        tree.add_leaf(
                            app.label(),
                            |_siv: &mut cursive::Cursive| {},
                        );
                    }
                })
                .delimiter()
                .leaf("Quit", cursive::Cursive::quit),
        );
    }

    {
        let screen = siv.screen_mut();
        for app in &vec_app {
            screen.add_layer(
                app.view(&title)
                    .scrollable()
                    .scroll_x(true)
                    .scroll_y(true)
                    .with_name(app.index.to_string())
            );

            if app.app_amdgpu_top.device_path.pci == selected_device_path.pci {
                toggle_opt.select_index = app.index;
            }
        }
        if let Some(pos) = screen.find_layer_from_name(&toggle_opt.select_index.to_string()) {
            screen.move_to_front(pos);
        }
    }

    let mut flags = toggle_opt.clone();
    let toggle_opt = Arc::new(Mutex::new(toggle_opt));

    siv.set_autohide_menu(false);
    siv.set_user_data(toggle_opt.clone());
    siv.set_theme(if is_dark_mode { dark_mode() } else { Theme::default() });

    {
        if !no_pc {
            siv.add_global_callback('g', pc_type_cb(PCType::GRBM));
            siv.add_global_callback('r', pc_type_cb(PCType::GRBM2));
        }
        siv.add_global_callback('v', VramUsageView::cb);
        siv.add_global_callback('a', ActivityView::cb);
        siv.add_global_callback('f', AppTextView::cb);
        siv.add_global_callback('R', AppTextView::cb_reverse_sort);
        siv.add_global_callback('P', AppTextView::cb_sort_by_pid);
        siv.add_global_callback('V', AppTextView::cb_sort_by_vram);
        siv.add_global_callback('C', AppTextView::cb_sort_by_cpu);
        siv.add_global_callback('G', AppTextView::cb_sort_by_gfx);
        siv.add_global_callback('M', AppTextView::cb_sort_by_media);
        siv.add_global_callback('n', AppTextView::cb_sensors);
        siv.add_global_callback('m', AppTextView::cb_gpu_metrics);
        siv.add_global_callback('q', cursive::Cursive::quit);
        siv.add_global_callback('h', |siv| {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.high_freq ^= true;
        });
        siv.add_global_callback('T', |siv| {
            let is_dark_mode;
            {
                let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
                opt.is_dark_mode ^= true;
                is_dark_mode = opt.is_dark_mode;
            }

            siv.set_theme(if is_dark_mode { dark_mode() } else { Theme::default() });
        });
        siv.add_global_callback(Key::Esc, |siv| siv.select_menubar());
    }

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || loop {
        {
            let lock = toggle_opt.try_lock();
            if let Ok(opt) = lock {
                flags = opt.clone();
            }
        }

        let sample = if flags.high_freq { Sampling::high() } else { Sampling::low() };

        {
            let selected_app = vec_app
                .iter_mut()
                .find(|app| flags.select_index == app.index)
                .unwrap();

            if !no_pc {
                for _ in 0..sample.count {
                    selected_app.app_amdgpu_top.update_pc();

                    std::thread::sleep(sample.delay);
                }
            } else {
                std::thread::sleep(sample.to_duration());
            }

            selected_app.update(&flags, &sample);
            if !no_pc { selected_app.app_amdgpu_top.clear_pc(); }
        }

        vec_sus_app.retain(|sus_app| {
            let is_active = sus_app.device_path.check_if_device_is_active();

            if is_active {
                let title = title.clone();
                let Some(tui_app) = sus_app.to_tui_app() else { return true };
                let index = tui_app.index;
                let label = tui_app.label();
                let info_bar = tui_app.app_amdgpu_top.device_info.info_bar();
                let stat = tui_app.app_amdgpu_top.stat.clone();
                let xdna_device_path = tui_app.app_amdgpu_top.xdna_device_path.clone();
                let app_layout = tui_app.layout.clone();

                vec_app.push(tui_app);

                cb_sink.send(Box::new(move |siv| {
                    {
                        let view = app_layout
                            .view(&title, info_bar, &stat, &xdna_device_path)
                            .scrollable()
                            .scroll_x(true)
                            .scroll_y(true)
                            .with_name(index.to_string());
                        let screen = siv.screen_mut();
                        let select_index = flags.select_index.to_string();
                        screen.add_layer(view);
                        if let Some(pos) = screen.find_layer_from_name(&select_index) {
                            screen.move_to_front(pos);
                        }
                    }

                    let menubar = siv.menubar();
                    let subtree = menubar.get_subtree(0).unwrap();
                    let len = subtree.len();
                    subtree.remove(len-3);

                    subtree.insert_leaf(
                        len-3,
                        label,
                        move |siv: &mut cursive::Cursive| {
                            let screen = siv.screen_mut();
                            let Some(pos) = screen.find_layer_from_name(&index.to_string())
                                else { return };
                            screen.move_to_front(pos);

                            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
                            opt.select_index = index;
                        },
                    );
                })).unwrap();
            }

            !is_active
        });

        cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
    });

    siv.run();
}

fn dark_mode() -> Theme {
    Theme {
            shadow: true,
            borders: BorderStyle::Simple,
            palette: Palette::terminal_default().with(|palette| {
                use cursive::theme::PaletteColor::*;
                use cursive::theme::BaseColor;

                palette[Background] = BaseColor::Black.light();

                palette[View] = BaseColor::Black.dark();
                palette[Primary] = BaseColor::White.dark();
                palette[TitlePrimary] = BaseColor::Cyan.light();

                palette[Highlight] = BaseColor::Cyan.light();
                palette[HighlightInactive] = BaseColor::Cyan.dark();
                palette[HighlightText] = BaseColor::Black.dark();
            }),
    }
}
