use libamdgpu_top::{DevicePath, PCI};
use libamdgpu_top::AMDGPU::DeviceHandle;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
#[cfg(feature = "git_version")]
const TITLE: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"), " (git-", env!("HEAD_ID"), ")");
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

    match main_opt.dump_mode {
        DumpMode::Info => {
            dump_info::dump(TITLE, &amdgpu_dev);
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
                    amdgpu_dev,
                    &device_path_list,
                    main_opt.update_process_index
                )
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("\"tui\" feature is not enabled for this build.");
                dump_info::dump(TITLE, &amdgpu_dev);
            }
        },
        #[cfg(feature = "gui")]
        AppMode::GUI => amdgpu_top_gui::run(
            APP_NAME,
            TITLE,
            &device_path_list,
            instance,
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
        let Some(instance) = device_path.get_instance_number() else { continue };

        println!("#{instance}");

        dump_info::dump(TITLE, &amdgpu_dev);

        println!("{device_path:?}\n");
    }
}

pub fn from_main_opt(main_opt: &MainOpt, list: &[DevicePath]) -> (DevicePath, DeviceHandle) {
    let device_path = if let Some(pci_path) = &main_opt.pci_path {
        let pci = pci_path.parse::<PCI::BUS_INFO>().unwrap_or_else(|_| {
            eprintln!("Failed to parse from {pci_path:?} to `PCI::BUS_INFO`");
            panic!();
        });

        DevicePath::try_from(pci).unwrap_or_else(|err| {
            eprintln!("{err}");
            eprintln!("pci_path: {pci_path:?}");
            eprintln!("Device list: {list:#?}");
            panic!();
        })
    } else {
        DevicePath::new(main_opt.instance)
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
