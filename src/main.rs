use libamdgpu_top::DevicePath;
use libamdgpu_top::AMDGPU::DeviceHandle;

const TITLE: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

mod args;
use args::{AppMode, MainOpt};
mod dump_info;

fn main() {
    let main_opt = MainOpt::parse();
    let device_path_list = DevicePath::get_device_path_list();

    if main_opt.list {
        device_list(AppMode::Dump == main_opt.app_mode, &device_path_list);
        return;
    }

    let (device_path, amdgpu_dev) = from_main_opt(&main_opt, &device_path_list);

    match main_opt.app_mode {
        AppMode::TUI => {
            #[cfg(feature = "tui")]
            {
                amdgpu_top_tui::run(TITLE, device_path, amdgpu_dev, main_opt.update_process_index)
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("\"tui\" feature is not enabled for this build.");
                dump_info::dump(&amdgpu_dev);
            }
        },
        #[cfg(feature = "gui")]
        AppMode::GUI => amdgpu_top_gui::run(
            TITLE,
            device_path,
            amdgpu_dev,
            &device_path_list,
            main_opt.update_process_index,
        ),
        #[cfg(feature = "json")]
        AppMode::JSON => amdgpu_top_json::run(
            device_path,
            amdgpu_dev,
            1000, // 1s
            main_opt.update_process_index,
        ),
        AppMode::Dump => dump_info::dump(&amdgpu_dev),
    }
}

pub fn device_list(dump_info: bool, list: &[DevicePath]) {
    for device_path in list {
        let Ok(amdgpu_dev) = device_path.init() else { continue };
        let Some(instance) = device_path.get_instance_number() else { continue };

        println!("#{instance}");

        if dump_info {
            dump_info::dump(&amdgpu_dev);
        } else {
            if let Ok(mark_name) = amdgpu_dev.get_marketing_name() {
                println!("Marketing Name = {mark_name:?}");
            }
        }
        println!("{device_path:?}");
        println!();
    }
}

pub fn from_main_opt(main_opt: &MainOpt, list: &[DevicePath]) -> (DevicePath, DeviceHandle) {
    // default
    if main_opt.instance == 0 && main_opt.pci_path.is_none() {
        return DevicePath::init_with_fallback(main_opt.instance, &main_opt.pci_path, list);
    }

    let device_path = if let Some(ref pci_path) = main_opt.pci_path {
        DevicePath::from_pci(pci_path).unwrap_or_else(|err| {
            eprintln!("{err}");
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
