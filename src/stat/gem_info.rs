use super::{Text, Opt};

/* ref: drivers/gpu/drm/amd/amdgpu/amdgpu_gem.c */
/* ref: drivers/gpu/drm/amd/amdgpu/amdgpu_object.c */

const MIB: u64 = 1 << 20;

#[derive(Debug, Default, Clone)]
struct GemInfo {
    pid: u32,
    vram_usage: u64,
    gtt_usage: u64,
    command_name: String,
}

#[derive(Default)]
pub struct GemView {
    raw: String,
    vec_gem: Vec<GemInfo>,
    pub text: Text,
}

impl GemView {
    pub fn clear(&mut self) {
        self.raw.clear();
        self.vec_gem.clear();
        self.text.clear();
    }

    pub fn read_to_print(&mut self, f: &mut std::fs::File) {
        self.clear();
        self.read_to_string(f);
        self.parse_raw_file();
        self.print();
    }

    pub fn read_to_string(&mut self, f: &mut std::fs::File) {
        use std::io::Read;

        f.read_to_string(&mut self.raw).unwrap();
    }

    pub fn parse_raw_file(&mut self) {
        let mut gem;
        let mut lines = self.raw.lines().peekable();

        'main: loop {
            gem = GemInfo::default();

            let line = match lines.next() {
                Some(line) => line,
                None => break 'main,
            };

            /* "pid     1479 command Xorg:" */
            /* "pid %8d command %s:\n" */
            if line.starts_with("pid") {
                const PID_RANGE: std::ops::Range<usize> = {
                    const PID_START: usize = 4;
                    const PID_LEN: usize = 8;

                    PID_START..(PID_START+PID_LEN)
                };
                const COMMAND_NAME: std::ops::RangeFrom<usize> = {
                    const COMMAND_NAME_START: usize = PID_RANGE.end + 9;

                    COMMAND_NAME_START..
                };

                gem.pid = line[PID_RANGE].trim_start().parse().unwrap();
                gem.command_name = line[COMMAND_NAME].to_string();

                if gem.command_name == concat!(env!("CARGO_PKG_NAME"), ":") {
                    continue;
                }
            } else {
                continue;
            }

            'calc_usage: loop {
                let mem_line = *lines.peek().unwrap_or(&"pid");

                if mem_line.starts_with("pid") {
                    if MIB * 10 < gem.vram_usage {
                        self.vec_gem.push(gem);
                    }
                    break 'calc_usage;
                }

                let _ = lines.next();

                /* "		0x00000001:      2097152 byte VRAM NO_CPU_ACCESS CPU_GTT_USWC" */
                /* "\t\t0x%08x: %12lld byte %s" */
                const USAGE_RANGE: std::ops::Range<usize> = {
                    const USAGE_START: usize = 4 + 8 + 2;
                    const USAGE_LEN: usize = 12;

                    USAGE_START..(USAGE_START+USAGE_LEN)
                };
                const MEM_TYPE_RANGE: std::ops::Range<usize> = {
                    const MEM_TYPE_START: usize = USAGE_RANGE.end + 6;
                    const MEM_TYPE_LEN: usize = 4;

                    MEM_TYPE_START..(MEM_TYPE_START+MEM_TYPE_LEN)
                };

                let byte: u64 = mem_line[USAGE_RANGE].trim_start().parse().unwrap();
                match &mem_line[MEM_TYPE_RANGE] {
                    "VRAM" => gem.vram_usage += byte,
                    " GTT" => gem.gtt_usage += byte,
                    " CPU" | _ => {},
                }
            } // 'calc_usage
        } // 'main
    }

    pub fn print(&mut self) {
        use std::fmt::Write;

        for g in &self.vec_gem {
            writeln!(
                self.text.buf,
                " {command_name:<20}({pid:>8}): {vram_usage:5} MiB VRAM, {gtt_usage:5} MiB GTT ",
                command_name = g.command_name,
                pid = g.pid,
                vram_usage = g.vram_usage >> 20, // MiB
                gtt_usage = g.gtt_usage >> 20, // MiB
            )
            .unwrap();
        }
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.gem ^= true;
        }
    }
}
