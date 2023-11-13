use libamdgpu_top::PCI;

pub struct MainOpt {
    pub instance: Option<usize>, // index
    pub refresh_period: u64, // ms
    pub update_process_index: u64, // sec
    pub pci: Option<PCI::BUS_INFO>,
    pub select_apu: bool,
    pub json_iterations: u32,
    pub app_mode: AppMode,
    pub dump_mode: DumpMode,
    pub single_gpu: bool,
    pub no_pc: bool,
}

impl Default for MainOpt {
    fn default() -> Self {
        Self {
            instance: None,
            refresh_period: 1000, // 1000ms, 1s
            update_process_index: 5, // sec
            pci: None,
            dump_mode: DumpMode::NoDump,
            select_apu: false,
            app_mode: AppMode::TUI,
            json_iterations: 0,
            single_gpu: false,
            no_pc: false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum AppMode {
    TUI,
    #[cfg(feature = "gui")]
    GUI,
    #[cfg(feature = "json")]
    JSON,
    #[cfg(feature = "tui")]
    SMI,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DumpMode {
    Info,
    List,
    Process,
    Version,
    NoDump,
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
    "       Dump AMDGPU info. (Specifications, VRAM, PCI, ResizableBAR, VBIOS, Video caps)\n",
    "       This option can be combined with the \"-J\" option.\n",
    "   --list\n",
    "       Display a list of AMDGPU devices.\n",
    "   -J, --json\n",
    "       Output JSON formatted data.\n",
    "       This option can be combined with the \"-d\" option.\n",
    "   --gui\n",
    "       Launch GUI mode.\n",
    "   --smi\n",
    "       Launch Simple TUI mode. (like nvidia-smi, rocm-smi)\n",
    "   -p, --process\n",
    "       Dump All GPU processes and memory usage per process.\n",
    "   --apu, --select-apu\n",
    "       Select APU instance.\n",
    "   --single, --single-gpu\n",
    "       Display only the selected APU/GPU\n",
    "   --no-pc\n",
    "       The application does not read the performance counter (GRBM, GRBM2).\n",
    "       Reading the performance counter may deactivate the power saving feature of APU/GPU.\n",
    "   -V, --version\n",
    "       Print version information.\n",
    "   -h, --help\n",
    "       Print help information.\n",
    "\n",
    "OPTIONS:\n",
    "   -i <usize>\n",
    "       Select GPU instance.\n",
    "   --pci <String>\n",
    "       Specifying PCI path. (domain:bus:dev.func)\n",
    "   -s <u64>, -s <u64>ms\n",
    "       Refresh period (interval) in milliseconds for JSON mode. (default: 1000ms)\n",
    "   -n <u32>\n",
    "       Specifies the maximum number of iteration for JSON mode.\n",
    "       If 0 is specified, it will be an infinite loop. (default: 0)\n",
    "   -u <u64>, --update-process-index <u64>\n",
    "       Update interval in seconds of the process index for fdinfo. (default: 5s)\n",
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
                        opt.instance = Some(val_str.parse::<usize>().unwrap());
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-i <usize>\"");
                        std::process::exit(1);
                    }
                },
                "-d" | "--dump" => {
                    opt.dump_mode = DumpMode::Info;
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
                "-s" => {
                    if let Some(val_str) = args.get(idx+1) {
                        let tmp = if val_str.ends_with("ms") {
                            let len = val_str.len();
                            val_str[..len-2].parse::<u64>().unwrap()
                        } else {
                            val_str.parse::<u64>().unwrap()
                        };

                        if tmp != 0 {
                            opt.refresh_period = tmp;
                        }

                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-s <u64>\"");
                        std::process::exit(1);
                    }
                },
                "-u" | "--update-process-index" => {
                    if let Some(val_str) = args.get(idx+1) {
                        let tmp = val_str.parse::<u64>().unwrap();

                        if tmp != 0 {
                            opt.update_process_index = tmp;
                        }

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
                    #[cfg(feature = "tui")]
                    {
                        opt.app_mode = AppMode::SMI;
                    }
                    #[cfg(not(feature = "tui"))]
                    {
                        eprintln!("\"tui\" feature is not enabled for this build.");
                        std::process::exit(1);
                    }
                },
                "--pci" => {
                    let s = args.get(idx+1).unwrap_or_else(|| {
                        eprintln!("missing argument: \"--pci <String>\"");
                        std::process::exit(1);
                    });
                    opt.pci = {
                        let pci = s.parse::<PCI::BUS_INFO>().unwrap_or_else(|_| {
                            eprintln!("Failed to parse from {s:?} to `PCI::BUS_INFO`");
                            std::process::exit(1);
                        });
                        Some(pci)
                    };
                    skip = true;
                },
                "-l" | "--list" => {
                    opt.dump_mode = DumpMode::List;
                },
                "--apu" | "--select-apu" => {
                    opt.select_apu = true;
                },
                "-n" => {
                    if let Some(val_str) = args.get(idx+1) {
                        opt.json_iterations = val_str.parse::<u32>().unwrap();
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-n <u32>\"");
                        std::process::exit(1);
                    }
                },
                "-V" | "--version" => {
                    opt.dump_mode = DumpMode::Version;
                },
                "-h" | "--help" => {
                    println!("{HELP_MSG}");
                    std::process::exit(0);
                },
                "-p" | "--process" => {
                    opt.dump_mode = DumpMode::Process;
                },
                "--single" | "--single-gpu" => {
                    opt.single_gpu = true;
                },
                "--no-pc" => {
                    opt.no_pc = true;
                },
                _ => {
                    eprintln!("Unknown option: {arg}");
                    std::process::exit(1);
                },
            }
        }

        opt
    }
}
