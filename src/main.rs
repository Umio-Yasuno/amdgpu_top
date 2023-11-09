use libamdgpu_top::DevicePath;
use libamdgpu_top::AMDGPU::DeviceHandle;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
#[cfg(feature = "git_version")]
const TITLE: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"), env!("HEAD_ID"));
#[cfg(not(feature = "git_version"))]
const TITLE: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

mod args;
use args::{AppMode, DumpMode, MainOpt};
mod dump_info;

fn main() {
    let main_opt = MainOpt::parse();
    let device_path_list = DevicePath::get_device_path_list();

    if device_path_list.is_empty() {
        eprintln!("There are no the AMD GPU devices found.");
        panic!();
    }

    #[cfg(feature = "json")]
    match (main_opt.app_mode, main_opt.dump_mode) {
        (AppMode::JSON, DumpMode::Info) => {
            amdgpu_top_json::dump_json(&device_path_list);
            return;
        },
        (AppMode::JSON, DumpMode::Version) => {
            amdgpu_top_json::version_json(TITLE);
            return;
        },
        (AppMode::JSON, DumpMode::NoDump) => {
            let mut j = amdgpu_top_json::JsonApp::new(
                &device_path_list,
                main_opt.refresh_period,
                main_opt.update_process_index,
                main_opt.json_iterations,
            );

            j.run(TITLE);

            return;
        },
        (_, _) => {},
    }

    let (device_path, amdgpu_dev) = if main_opt.select_apu {
        select_apu(&device_path_list)
    } else {
        from_main_opt(&main_opt, &device_path_list)
    };
    let instance = device_path.instance_number;

    match main_opt.dump_mode {
        DumpMode::Info => {
            dump_info::dump(TITLE, &amdgpu_dev, instance);
            return;
        },
        DumpMode::List => {
            device_list(&device_path_list);
            return;
        },
        DumpMode::Process => {
            dump_info::dump_process(TITLE, &device_path_list);
            return;
        },
        DumpMode::Version => {
            println!("{TITLE}");
            return;
        },
        DumpMode::NoDump => {},
    }

    match main_opt.app_mode {
        AppMode::TUI => {
            #[cfg(feature = "tui")]
            {
                amdgpu_top_tui::run(
                    TITLE,
                    device_path,
                    &device_path_list,
                    main_opt.update_process_index
                )
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("\"tui\" feature is not enabled for this build.");
                dump_info::dump(TITLE, &amdgpu_dev, instance);
            }
        },
        #[cfg(feature = "gui")]
        AppMode::GUI => amdgpu_top_gui::run(
            APP_NAME,
            TITLE,
            &device_path_list,
            device_path.pci,
            main_opt.update_process_index,
        ),
        #[cfg(feature = "json")]
        AppMode::JSON => unreachable!(),
        #[cfg(feature = "tui")]
        AppMode::SMI => amdgpu_top_tui::run_smi(
            TITLE,
            &device_path_list,
            main_opt.update_process_index,
        ),
    }
}

pub fn device_list(list: &[DevicePath]) {
    for device_path in list {
        let Ok(amdgpu_dev) = device_path.init() else { continue };
        let instance = device_path.instance_number;

        println!("#{instance}");

        dump_info::dump(TITLE, &amdgpu_dev, instance);

        println!("{device_path:?}\n");
    }
}

pub fn from_main_opt(main_opt: &MainOpt, list: &[DevicePath]) -> (DevicePath, DeviceHandle) {
    let device_path = if let Some(pci) = main_opt.pci {
        DevicePath::try_from(pci).unwrap_or_else(|err| {
            eprintln!("{err}");
            eprintln!("pci_path: {pci:?}");
            eprintln!("Device list: {list:#?}");
            panic!();
        })
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
        list.iter().next().unwrap().clone()
    };

    let amdgpu_dev = device_path.init().unwrap_or_else(|err| {
        eprintln!("{err}");
        eprintln!("{:?}", device_path);
        eprintln!("Device list: {list:#?}");
        panic!();
    });

    (device_path, amdgpu_dev)
}

fn select_apu(list: &[DevicePath]) -> (DevicePath, DeviceHandle) {
    use libamdgpu_top::AMDGPU::GPU_INFO;

    for device_path in list {
        let Ok(amdgpu_dev) = device_path.init() else { continue };
        let Ok(ext_info) = amdgpu_dev.device_info() else { continue };

        if ext_info.is_apu() {
            return (device_path.clone(), amdgpu_dev);
        }
    }

    eprintln!("The APU device is not installed or disabled.");
    panic!();
}
