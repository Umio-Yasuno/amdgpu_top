#[derive(Debug, Clone)]
pub(crate) struct GemInfo {
    pid: u32,
    vram_usage: u64,
    gtt_usage: u64,
    command_name: String,
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
        let mut gem;
        let mut lines = self.raw.lines().peekable();

        'main: loop {
            gem = GemInfo::default();

            let line = match lines.next() {
                Some(line) => line,
                None => break 'main,
            };

            /* pid     1479 command Xorg: */
            if line.starts_with("pid") {
                let tmp: Vec<&str> = line.split(' ').collect();
                gem.pid = tmp[tmp.len() - 3].parse().unwrap();
                gem.command_name = tmp[tmp.len() - 1].to_string();
            } else {
                continue;
            }

            'calc_usage: loop {
                let mem_line = match lines.peek() {
                    Some(&mem_line) => mem_line,
                    None => {
                        self.vec_gem.push(gem);
                        break 'main;
                    },
                };

                if mem_line.starts_with("pid") {
                    self.vec_gem.push(gem);
                    break 'calc_usage;
                }

                let _ = lines.next();

                /* 		0x00000001:      2097152 byte VRAM NO_CPU_ACCESS CPU_GTT_USWC */
                let split: Vec<&str> = mem_line.split(" byte ").collect();
                let byte: u64 = split[0][13..].trim_start().parse().unwrap();
                match &split[1][..4] {
                    "VRAM" => gem.vram_usage += byte,
                    " GTT" => gem.gtt_usage += byte,
                    _ => {},
                }
            } // 'calc_usage
        } // 'main
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
                " {command_name:<15}({pid:>8}): {vram_usage:5} MiB VRAM, {gtt_usage:5} MiB GTT ",
                command_name = g.command_name,
                pid = g.pid,
                vram_usage = g.vram_usage >> 20, // MiB
                gtt_usage = g.gtt_usage >> 20, // MiB
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
