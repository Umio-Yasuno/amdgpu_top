#[derive(Default)]
pub struct MainOpt {
    pub instance: u32,
    pub dump: bool,
}

const HELP_MSG: &str = concat!(
    "amdgpu_top ", env!("CARGO_PKG_VERSION"), "\n",
    "https://github.com/Umio-Yasuno/amdgpu_top\n",
    "\n",
    "USAGE:\n",
    "    cargo run -- [options ..] or <amdgpu_top> [options ..]\n",
    "\n",
    "FLAGS:\n",
    "   -d, --dump\n",
    "       Dump AMDGPU info (name, clock, chip_class, VRAM, PCI, VBIOS)\n",
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
