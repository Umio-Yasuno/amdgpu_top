#[derive(Default)]
pub struct MainOpt {
    pub instance: u32,
    pub dump: bool,
    pub json_output: bool,
    pub refresh_period: u64, // ms
    pub pid: Option<i32>,
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
    "       Dump AMDGPU info (name, clock, chip_class, VRAM, PCI, VBIOS)\n",
    "   -J\n",
    "       Output JSON formatted data\n",
    "   -s <i64>, --ms <i64>\n",
    "       Refresh period in milliseconds\n",
    "   -p <i32>, --pid <i32>\n",
    "       Specification of PID, used for `-J` option\n",
    "\n",
    "OPTIONS:\n",
    "   -i <u32>\n",
    "       Select GPU instance\n",
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
                        opt.pid = Some(val_str.parse::<i32>().unwrap());
                        skip = true;
                    } else {
                        eprintln!("missing argument: \"-p <i32>\"");
                        std::process::exit(1);
                    }
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
