pub struct MainOpt {
    pub instance: u32,
    pub dump: bool,
    pub json_output: bool,
    pub refresh_period: u64, // ms
    pub pid: Option<i32>,
    pub update_process_index: u64, // sec
    pub gui: bool,
}

impl Default for MainOpt {
    fn default() -> Self {
        Self {
            instance: 0,
            dump: false,
            json_output: false,
            refresh_period: 500, // ms
            pid: None,
            update_process_index: 5, // sec
            gui: false,
        }
    }
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
    "   -J\n",
    "       Output JSON formatted data for simple process trace\n",
    "   --gui\n",
    "       Launch GUI mode\n",
    "   -h, --help\n",
    "       Print help information\n",
    "\n",
    "OPTIONS:\n",
    "   -i <u32>\n",
    "       Select GPU instance\n",
    "   -u <u64>, --update-process-index <u64>\n",
    "       Update interval in seconds of the process index for fdinfo (default: 5s)\n",
    "   -s <i64>, --ms <i64>\n",
    "       Refresh period in milliseconds for simple process trace\n",
    "   -p <i32>, --pid <i32>\n",
    "       Specification of PID, used for `-J` option\n",
);

impl MainOpt {
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
                    opt.dump = true;
                },
                "-J" => {
                    opt.json_output = true;
                },
                "-s" | "--ms" => {
                    if let Some(val_str) = args.get(idx+1) {
                        opt.refresh_period = val_str.parse::<u64>().unwrap();
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-s <u64>\"");
                        std::process::exit(1);
                    }
                },
                "-p" | "--pid" => {
                    if let Some(val_str) = args.get(idx+1) {
                        opt.pid = val_str.parse::<i32>().ok();
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-p <i32>\"");
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
                    opt.gui = true;
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
