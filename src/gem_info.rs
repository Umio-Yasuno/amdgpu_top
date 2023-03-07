#[derive(Debug, Clone)]
pub(crate) struct GemInfo {
    pub(crate) pid: u32,
    pub(crate) vram_usage: u64,
    pub(crate) gtt_usage: u64,
    pub(crate) command_name: String,
}

impl Default for GemInfo {
    fn default() -> Self {
        Self {
            pid: 0,
            vram_usage: 0,
            gtt_usage: 0,
            command_name: "".to_string(),
        }
    }
}

impl GemInfo {
    pub(crate) fn get(raw: &String, gem_vec: &mut Vec<Self>) {
        let mut gem;
        // let mut gem_vec: Vec<GemInfo> = Vec::new();
        // let mut lines = gem_info.lines().peekable();
        let mut lines = raw.lines().peekable();

        'main: loop {
            gem = GemInfo::default();

            let d = match lines.next() {
                Some(d) => d,
                None => break 'main,
            };

            if !d.starts_with("pid") {
                continue;
            }

            {
                let tmp: Vec<&str> = d.split(' ').collect();
                gem.pid = tmp[tmp.len() - 3].parse().unwrap();
                gem.command_name = tmp[tmp.len() - 1].to_string();
            }

            'calc_usage: loop {
                let d = match lines.peek() {
                    Some(&d) => d,
                    None => {
                        gem_vec.push(gem);
                        break 'main;
                    },
                };

                if d.starts_with("pid") {
                    gem_vec.push(gem);
                    break 'calc_usage;
                }

                let _ = lines.next();

                let split: Vec<&str> = d.split(" byte ").collect();
                let byte: u64 = split[0][13..].trim_start().parse().unwrap();
                match &split[1][..4] {
                    "VRAM" => gem.vram_usage += byte,
                    " GTT" => gem.gtt_usage += byte,
                    _ => {},
                }
            }
        }
    }
}

pub(crate) struct GemView {
    pub(crate) raw: String,
    pub(crate) vec_gem: Vec<GemInfo>,
    pub(crate) buf: String,
}

impl GemView {
    pub(crate) fn clear(&mut self) {
        self.raw.clear();
        self.vec_gem.clear();
        self.buf.clear();
    }

    pub(crate) fn set(&mut self, f: &mut std::fs::File) {
        self.clear();
        self.read_to_string(f);
        self.parse_raw_file();
        self.print();
    }

    pub(crate) fn read_to_string(&mut self, f: &mut std::fs::File) {
        use std::io::Read;

        f.read_to_string(&mut self.raw).unwrap();
    }

    pub(crate) fn parse_raw_file(&mut self) {
        GemInfo::get(&self.raw, &mut self.vec_gem)
    }

    pub(crate) fn print(&mut self) {
        use std::fmt::Write;
        const MIB: u64 = 1 << 20;

        for g in &self.vec_gem {
            if g.command_name == "amdgpu_top:" {
                continue;
            }
            if g.vram_usage < MIB {
                continue;
            }

            writeln!(
                self.buf,
                "{command_name:<10}({pid:^6}): {vram_usage:5} MiB VRAM, {gtt_usage:5} MiB GTT",
                command_name = g.command_name,
                pid = g.pid,
                vram_usage = g.vram_usage >> 20,
                gtt_usage = g.gtt_usage >> 20,
            ).unwrap();
        }
    }
}

impl Default for GemView {
    fn default() -> Self {
        Self {
            raw: String::new(),
            vec_gem: Vec::new(),
            buf: String::new(),
        }
    }
}
