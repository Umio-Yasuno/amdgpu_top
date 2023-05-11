pub struct MainOpt {
    pub instance: u32,
    pub pid: Option<i32>,
    pub update_process_index: u64, // sec
    pub pci_path: Option<String>,
    pub list: bool,
    pub app_mode: AppMode,
}

impl Default for MainOpt {
    fn default() -> Self {
        Self {
            instance: 0,
            pid: None,
            update_process_index: 5, // sec
            pci_path: None,
            list: false,
            app_mode: AppMode::TUI,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum AppMode {
    TUI,
    #[cfg(feature = "gui")]
    GUI,
    #[cfg(feature = "json")]
    JSON,
    SMI,
    Dump,
}

const HELP_MSG: &str = concat!(
    env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"), "\n",
    env!("CARGO_PKG_HOMEPAGE"), "\n",
    "\n",
    "USAGE:\n",
    "    cargo run -- [options ..] or <", env!("CARGO_PKG_NAME"), "> [options ..]\n",
    "\n",
    "FLAGS:\n",
    "   -d, --dump\n",
    "       Dump AMDGPU info (Specifications, VRAM, PCI, ResizableBAR, VBIOS, Video caps)\n",
    "   --list\n",
    "       Display a list of AMDGPU devices (can be combined with \"-d\" option)\n",
    "   -J, --json\n",
    "       Output JSON formatted data\n",
    "   --gui\n",
    "       Launch GUI mode\n",
    "   --smi\n",
    "       Launch Simple TUI mode (like nvidia-smi, rocm-smi)\n",
    "   -h, --help\n",
    "       Print help information\n",
    "\n",
    "OPTIONS:\n",
    "   -i <u32>\n",
    "       Select GPU instance\n",
    "   --pci <String>\n",
    "       Specifying PCI path (domain:bus:dev.func)\n",
    "   -u <u64>, --update-process-index <u64>\n",
    "       Update interval in seconds of the process index for fdinfo (default: 5s)\n",
);

impl MainOpt {
    #[allow(unused_assignments)]
    pub fn parse() -> Self {
        let mut opt = Self::default();
        let mut skip = false;

        let args = &std::env::args().collect::<Vec<String>>()[1..];

        for (idx, arg) in args.iter().enumerate() {
            if skip {
                skip = false;
                continue;
            }

            if !arg.starts_with('-') {
                eprintln!("Unknown option: {arg}");
                continue;
            }

            match arg.as_str() {
                "-i" => {
                    if let Some(val_str) = args.get(idx+1) {
                        opt.instance = val_str.parse::<u32>().unwrap();
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-i <u32>\"");
                        std::process::exit(1);
                    }
                },
                "-d" | "--dump" => {
                    opt.app_mode = AppMode::Dump;
                },
                "-J" | "--json" => {
                    #[cfg(feature = "json")]
                    {
                        opt.app_mode = AppMode::JSON;
                    }
                    #[cfg(not(feature = "json"))]
                    {
                        eprintln!("\"json\" feature is not enabled for this build.");
                        std::process::exit(1);
                    }
                },
                "-u" | "--update-process-index" => {
                    if let Some(val_str) = args.get(idx+1) {
                        let tmp = val_str.parse::<u64>().unwrap();
                        opt.update_process_index = if tmp == 0 { 1 } else { tmp };
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-u <u64>\"");
                        std::process::exit(1);
                    }
                },
                "--gui" => {
                    #[cfg(feature = "gui")]
                    {
                        opt.app_mode = AppMode::GUI;
                    }
                    #[cfg(not(feature = "gui"))]
                    {
                        eprintln!("\"gui\" feature is not enabled for this build.");
                        std::process::exit(1);
                    }
                },
                "--smi" => {
                    opt.app_mode = AppMode::SMI;
                },
                "--pci" => {
                    opt.pci_path = args.get(idx+1).map(|v| v.to_string());
                    skip = true;
                },
                "--list" => {
                    opt.list = true;
                },
                "-h" | "--help" => {
                    println!("{HELP_MSG}");
                    std::process::exit(0);
                },
                _ => {
                    eprintln!("Unknown option: {arg}")
                },
            }
        }

        opt
    }
}
