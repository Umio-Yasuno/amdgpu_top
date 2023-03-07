pub(crate) struct MainOpt {
    pub(crate) instance: u32,
    pub(crate) dump: bool,
}

impl Default for MainOpt {
    fn default() -> Self {
        Self {
            instance: 0,
            dump: false,
        }
    }
}

impl MainOpt {
    pub(crate) fn parse() -> Self {
        let mut opt = Self::default();
        let mut skip = false;

        let args: Vec<String> = std::env::args().collect();

        for (idx, arg) in args[1..].iter().enumerate() {
            if skip {
                skip = false;
                continue;
            }

            if !arg.starts_with('-') {
                continue;
            }

            match arg.as_str() {
                "-i" => {
                    if let Some(val_str) = args.get(idx+2) {
                        opt.instance = val_str.parse::<u32>().unwrap();
                        skip = true;
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
