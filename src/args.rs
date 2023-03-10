#[derive(Default)]
pub struct MainOpt {
    pub instance: u32,
    pub dump: bool,
}

impl MainOpt {
    pub fn parse() -> Self {
        let mut opt = Self::default();
        let mut skip = false;

        let args: Vec<String> = std::env::args().collect();
        let args = &args[1..];

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
                _ => {
                    eprintln!("Unknown option: {arg}")
                },
            }
        }

        opt
    }
}
