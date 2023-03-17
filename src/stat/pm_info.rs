use super::{Text, Opt};

/* ref: drivers/gpu/drm/amd/pm/amdgpu_pm.c */

#[derive(Default)]
pub struct PmView {
    raw: String,
    pub text: Text,
}

impl PmView {
    pub fn clear(&mut self) {
        self.raw.clear();
        self.text.clear();
    }

    pub fn read_to_print(&mut self, f: &mut std::fs::File) {
        self.clear();
        self.read_to_string(f);
        self.print();
    }

    pub fn read_to_string(&mut self, f: &mut std::fs::File) {
        use std::io::Read;

        f.read_to_string(&mut self.raw).unwrap();
    }

    pub fn print(&mut self) {
        use std::fmt::Write;
        
        /* "UVD: Disabled\n" or "UVD: Enabled\n" */
        const STATUS_RANGE: std::ops::Range<usize> = {
            5..8
        };

        let lines = self.raw.lines();

        for ln in lines {
            let len = ln.len();

            if len < 10 {
                continue;
            }

            if ln.starts_with('\t') {
                match &ln[(len-7)..len] {
                    " (DCLK)" | " (VCLK)" | "(ECCLK)" => {
                         writeln!(
                            self.text.buf,
                            "{ln}",
                        )
                        .unwrap();
                    },
                    _ => {},
                }
                continue;
            }

            let engine = &ln[..3];
            let stat = match engine {
                "UVD" | "VCE" | "VCN" => match &ln[STATUS_RANGE] {
                    "Dis" => "  Idle",
                    "Ena" => "Active",
                    _ => continue,
                },
                _ => continue,
            };

            writeln!(
                self.text.buf,
                " {engine}: {stat} ",
            )
            .unwrap();
        }
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.pm ^= true;
        }
    }
}
