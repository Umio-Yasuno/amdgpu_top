use libamdgpu_top::{DevicePath, UiArgs};

#[cfg(feature = "gui")]
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const TITLE: &str = env!("TITLE");

mod args;
use args::{AppMode, DumpMode, OptDumpMode, MainOpt};
mod dump_info;
mod dump_pp_table;
mod dump_process;
use dump_process::dump_process;
mod dump_xdna_device;
mod drm_info;

fn main() {
    let main_opt = MainOpt::parse();

    if let Some(path) = &main_opt.decode_gpu_metrics {
        let gm = dump_info::decode_gpu_metrics(path);

        #[cfg(feature = "json")]
        if let AppMode::JSON = main_opt.app_mode {
            use amdgpu_top_json::OutputJson;
            println!("{}", gm.json());
            return;
        }

        println!("{gm:#?}");
        return;
    }

    fn get_list_and_selected_device_path(main_opt: &MainOpt)
        -> (Vec<DevicePath>, DevicePath)
    {
        let mut list = DevicePath::get_device_path_list();

        if list.is_empty() {
            eprintln!("There are no the AMD GPU devices found.");
            panic!();
        }

        let selected_device_path = if main_opt.select_apu {
            select_apu(&list)
        } else {
            from_main_opt(&main_opt, &list)
        };

        if main_opt.single_gpu {
            return (vec![selected_device_path.clone()], selected_device_path);
        }

        let pos = list
            .iter()
            .position(|device_path| device_path.pci == selected_device_path.pci)
            .unwrap();
        list.remove(pos);
        list.insert(0, selected_device_path.clone());

        (list, selected_device_path)
    }

    let (device_path_list, device_path) = get_list_and_selected_device_path(&main_opt);

    #[cfg(feature = "json")]
    if let AppMode::JSON = main_opt.app_mode { match main_opt.dump_mode {
        DumpMode::Info => {
            amdgpu_top_json::dump_json(&device_path_list);
            return;
        },
        DumpMode::Version => {
            amdgpu_top_json::version_json(TITLE);
            return;
        },
        DumpMode::PPTable => {},
        DumpMode::NoDump => {
            match main_opt.opt_dump_mode {
                OptDumpMode::GpuMetrics => {
                    amdgpu_top_json::gpu_metrics_json(TITLE, &device_path_list);
                    return;
                },
                OptDumpMode::DrmInfo => {
                    amdgpu_top_json::drm_info_json(&device_path_list);
                    return;
                },
                _ => {},
            }

            let mut j = amdgpu_top_json::JsonApp::new(
                TITLE,
                &device_path_list,
                main_opt.refresh_period,
                main_opt.update_process_index,
                main_opt.json_iterations,
                main_opt.no_pc,
            );

            j.run();

            return;
        },
        _ => {},
    }}

    match main_opt.dump_mode {
        DumpMode::Info => {
            dump_info::dump_all(
                TITLE,
                &device_path_list,
                main_opt.opt_dump_mode,
            );
            return;
        },
        DumpMode::List => {
            device_list(&device_path_list);
            return;
        },
        DumpMode::Process => {
            dump_process(TITLE, &device_path_list);
            return;
        },
        DumpMode::Version => {
            println!("{TITLE}");
            return;
        },
        DumpMode::PPTable => {
            dump_pp_table::dump_all_pp_table(TITLE, &device_path_list);
            return;
        },
        DumpMode::Xdna => {
            dump_xdna_device::dump_xdna_device(TITLE);
            return;
        },
        DumpMode::NoDump => match main_opt.opt_dump_mode {
            OptDumpMode::GpuMetrics => {
                dump_info::dump_gpu_metrics(TITLE, &device_path_list);
                return;
            },
            OptDumpMode::DrmInfo => {
                drm_info::dump_all_drm_info(&device_path_list);
                return;
            },
            _ => {},
        },
    }

    let ui_args = UiArgs {
        selected_device_path: device_path,
        device_path_list,
        update_process_index: main_opt.update_process_index,
        no_pc: main_opt.no_pc,
        is_dark_mode: main_opt.is_dark_mode,
        hide_fdinfo: main_opt.hide_fdinfo,
        gui_wgpu_backend: main_opt.wgpu_backend,
        tab_gui: main_opt.tab_gui,
    };

    match main_opt.app_mode {
        AppMode::TUI => {
            #[cfg(feature = "tui")]
            {
                amdgpu_top_tui::run(TITLE, ui_args)
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("\"tui\" feature is not enabled for this build.");
                dump_info::dump(&ui_args.selected_device_path, main_opt.opt_dump_mode);
            }
        },
        #[cfg(feature = "gui")]
        AppMode::GUI => amdgpu_top_gui::run(APP_NAME, TITLE, ui_args),
        #[cfg(feature = "json")]
        AppMode::JSON => unreachable!(),
        #[cfg(feature = "json")]
        AppMode::JSON_FIFO(path_string) => {
            use std::ffi::CString;
            use std::path::PathBuf;

            let path = PathBuf::from(path_string.clone());

            if path.exists() {
                panic!("{path:?} already exists");
            };

            {
                let bytes = path_string.as_bytes();
                let fifo_path = CString::new(bytes).unwrap();

                let r = unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o644) };

                if r != 0 {
                    panic!("mkfifo failed: {r}, {path:?}");
                }
            }

            let mut j = amdgpu_top_json::JsonApp::new(
                TITLE,
                &ui_args.device_path_list,
                main_opt.refresh_period,
                ui_args.update_process_index,
                main_opt.json_iterations,
                ui_args.no_pc,
            );

            j.run_fifo(path);
        },
        #[cfg(feature = "tui")]
        AppMode::SMI => amdgpu_top_tui::run_smi(TITLE, ui_args),
    }
}

pub fn device_list(list: &[DevicePath]) {
    println!("{TITLE}\n");
    for (i, device_path) in list.iter().enumerate() {
        println!("#{i}:");
        println!("{device_path:#X?}");
    }
}

pub fn from_main_opt(main_opt: &MainOpt, list: &[DevicePath]) -> DevicePath {
    if let Some(pci) = main_opt.pci {
        list.iter().find(|device_path| device_path.pci == pci).unwrap().clone()
    } else if let Some(i) = main_opt.instance {
        list
            .get(i)
            .unwrap_or_else(|| {
                eprintln!("index out of bounds: {i}");
                for (i, device) in list.iter().enumerate() {
                    eprintln!("#{i}: {device:#?}");
                }
                panic!();
            })
            .clone()
    } else {
        list.get(0).unwrap().clone()
    }
}

fn select_apu(list: &[DevicePath]) -> DevicePath {
    use libamdgpu_top::AMDGPU::GPU_INFO;

    list.iter().find(|&device_path| {
        if !device_path.check_if_device_is_active() {
            return false;
        }

        let Ok(amdgpu_dev) = device_path.init() else { return false };
        let Ok(ext_info) = amdgpu_dev.device_info() else { return false };

        ext_info.is_apu()
    }).unwrap_or_else(|| {
        eprintln!("The APU device is not installed or disabled.");
        panic!();
    }).clone()
}
