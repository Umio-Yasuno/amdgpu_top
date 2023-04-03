use std::fs;
use std::io::Read;
use std::fmt::Write;
use super::{Text, Opt};
// use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_fdinfo.c

const PROC_NAME_LEN: usize = 15;

const VRAM_LABEL: &str = "VRAM";
const GFX_LABEL: &str = "GFX";
const COMPUTE_LABEL: &str = "Compute";
const DMA_LABEL: &str = "DMA";
const DEC_LABEL: &str = "DEC";
const ENC_LABEL: &str = "ENC";
// const UVD_ENC_LABEL: &str = "UVD (ENC)";
// const JPEG_LABEL: &str = "JPEG";

#[derive(Debug, Default, Clone)]
pub struct ProcInfo {
    pid: i32,
    name: String,
    fds: Vec<i32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct FdInfoUsage {
    // client_id: usize,
    vram_usage: u64, // KiB
    gtt_usage: u64, // KiB
    cpu_accessible_usage: u64, // KiB
    gfx: i64,
    compute: i64,
    dma: i64,
    dec: i64,
    enc: i64,
    uvd_enc: i64,
    vcn_jpeg: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct ProcUsage {
    pid: i32,
    name: String,
    usage: FdInfoUsage,
}

#[derive(Default)]
pub struct FdInfoView {
    pid_map: HashMap<i32, FdInfoUsage>,
    proc_usage: Vec<ProcUsage>,
    pub interval: Duration,
    pub text: Text,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FdInfoSortType {
    PID,
    VRAM,
    GFX,
}

impl FdInfoView {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            ..Default::default()
        }
    }

    pub fn print(&mut self, slice_proc_info: &[ProcInfo], sort: &FdInfoSortType, reverse: bool) {
        self.text.clear();
        self.proc_usage.clear();

        writeln!(
            self.text.buf,
            " {pad:26} | {VRAM_LABEL:^8} | {GFX_LABEL} | {COMPUTE_LABEL} | {DMA_LABEL} | {DEC_LABEL} | {ENC_LABEL} |",
            pad = "",
        ).unwrap();

        for proc_info in slice_proc_info {
            self.get_proc_usage(proc_info);
        }

        self.proc_usage.sort_by(|a, b|
            match (sort, reverse) {
                (FdInfoSortType::PID, false) => b.pid.cmp(&a.pid),
                (FdInfoSortType::PID, true) => a.pid.cmp(&b.pid),
                (FdInfoSortType::VRAM, false) => b.usage.vram_usage.cmp(&a.usage.vram_usage),
                (FdInfoSortType::VRAM, true) => a.usage.vram_usage.cmp(&b.usage.vram_usage),
                (FdInfoSortType::GFX, false) => b.usage.gfx.cmp(&a.usage.gfx),
                (FdInfoSortType::GFX, true) => a.usage.gfx.cmp(&b.usage.gfx),
            }
        );

        self.print_usage();
    }

    pub fn print_usage(&mut self) {
        for pu in &self.proc_usage {
            write!(
                self.text.buf,
                " {name:PROC_NAME_LEN$} ({pid:>8}) | {vram:>5} MiB|",
                name = pu.name,
                pid = pu.pid,
                vram = pu.usage.vram_usage >> 10,
            ).unwrap();
            let enc_usage = pu.usage.enc + pu.usage.uvd_enc;
            for (usage, label_len) in [
                (pu.usage.gfx, GFX_LABEL.len()),
                (pu.usage.compute, COMPUTE_LABEL.len()),
                (pu.usage.dma, DMA_LABEL.len()),
                (pu.usage.dec, DEC_LABEL.len()), // UVD/VCN
                (enc_usage, ENC_LABEL.len()),
                // (enc, ENC_LABEL), // VCE/VCN
                // (uvd_enc, UVD_ENC_LABEL), // UVD
                // (vcn_jpeg, JPEG_LABEL) // VCN
            ] {
                write!(self.text.buf, " {usage:>label_len$}%|").unwrap();
            }
            writeln!(self.text.buf).unwrap();
        }
    }

    pub fn get_proc_usage(&mut self, proc_info: &ProcInfo) {
        let pid = proc_info.pid;
        let name = if PROC_NAME_LEN < proc_info.name.len() {
            &proc_info.name[..PROC_NAME_LEN]
        } else {
            &proc_info.name
        };
        let mut ids = HashSet::<usize>::new();
        let mut stat = FdInfoUsage::default();
        let mut buf = String::new();

        'fds: for fd in &proc_info.fds {
            let path = format!("/proc/{pid}/fdinfo/{fd}");
            let Ok(mut f) = fs::File::open(&path) else { continue; };
            if let Err(_) = f.read_to_string(&mut buf) { continue; }
            let mut lines = buf.lines();

            'fdinfo: loop {
                let Some(l) = lines.next() else { break 'fdinfo; };

                /* // perf
                    let id = id_parse(l);
                    if !ids.insert(id) { continue 'fds; }
                    if !l.starts_with("drm-client-id") { continue 'fdinfo; }
                    stat.vram_usage += mem_parse(lines.next().unwrap_or(""));
                    stat.gtt_usage += mem_parse(lines.next().unwrap_or(""));
                    stat.cpu_accessible_usage += mem_parse(lines.next().unwrap_or(""));

                    'engines: loop {
                        let Some(e) = lines.next() else { break 'engines; };
                        if !e.starts_with("drm-engine") { continue 'engines; }
                        stat.engine_parse(e);
                    }
                */
                if l.starts_with("drm-client-id") {
                    let id = FdInfoUsage::id_parse(l);
                    if !ids.insert(id) { continue 'fds; }
                    continue 'fdinfo;
                }
                if l.starts_with("drm-memory") {
                    stat.mem_usage_parse(l);
                    continue 'fdinfo;
                }
                if l.starts_with("drm-engine") {
                    stat.engine_parse(l);
                    continue 'fdinfo;
                }
            } // 'fdinfo
        } // 'fds

        let diff = {
            if let Some(pre_stat) = self.pid_map.get_mut(&pid) {
                let tmp = stat.calc_usage(pre_stat, &self.interval);
                *pre_stat = stat;

                tmp
            } else {
                let [vram_usage, gtt_usage, cpu_accessible_usage] = [
                    stat.vram_usage,
                    stat.gtt_usage,
                    stat.cpu_accessible_usage,
                ];

                self.pid_map.insert(pid, stat);

                FdInfoUsage {
                    vram_usage,
                    gtt_usage,
                    cpu_accessible_usage,
                    ..Default::default()
                }
            }
        };

        self.proc_usage.push(ProcUsage {
            pid,
            name: name.to_string(),
            usage: diff
        });
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo ^= true;
        }
    }

    pub fn cb_reverse_sort(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.reverse_sort ^= true;
        }
    }

    pub fn cb_sort_by_pid(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::PID;
        }
    }

    pub fn cb_sort_by_vram(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::VRAM;
        }
    }

    pub fn cb_sort_by_gfx(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::GFX;
        }
    }
}

impl FdInfoUsage {
    fn id_parse(s: &str) -> usize {
        const LEN: usize = "drm-client-id:\t".len();
        s[LEN..].parse().unwrap()
    }

    fn mem_usage_parse(&mut self, s: &str) {
        const PREFIX: usize = "drm-memory-xxxx:\t".len(); // "vram" or "gtt " or "cpu "
        const SUFFIX: usize = " KiB".len();
        let len = s.len();

        const MEM_TYPE: std::ops::Range<usize> = {
            const PRE_LEN: usize = "drm-memory-".len();

            PRE_LEN..(PRE_LEN+5)
        };

        let usage = s[PREFIX..len-SUFFIX].parse().unwrap_or(0);

        match &s[MEM_TYPE] {
            "vram:" => self.vram_usage += usage,
            "gtt: " => self.gtt_usage += usage,
            "cpu: " => self.cpu_accessible_usage += usage,
            _ => {},
        };
    }

    fn engine_parse(&mut self, s: &str) {
        const PRE: usize = "drm-engine-".len();
        const NS: usize = " ns".len();
        let pos = s.find('\t').unwrap();

        let ns: i64 = {
            let len = s.len();
            s[pos+1..(len-NS)].parse().unwrap_or(0)
        };

        match &s[PRE..pos] {
            "gfx:" => self.gfx += ns,
            "compute:" => self.compute += ns,
            "dma:" => self.dma += ns,
            "dec:" => self.dec += ns,
            "enc:" => self.enc += ns,
            "enc_1:" => self.uvd_enc += ns,
            "jpeg:" => self.vcn_jpeg += ns,
            _ => {},
        };
    }

    pub fn calc_usage(&self, pre_stat: &Self, interval: &Duration) -> Self {
        let [gfx, compute, dma, dec, enc, uvd_enc, _vcn_jpeg] = {
            [
                (pre_stat.gfx, self.gfx),
                (pre_stat.compute, self.compute),
                (pre_stat.dma, self.dma),
                (pre_stat.dec, self.dec),
                (pre_stat.enc, self.enc),
                (pre_stat.uvd_enc, self.uvd_enc),
                (pre_stat.vcn_jpeg, self.vcn_jpeg),
            ]
            .map(|(pre, cur)| {
                let usage = if pre == 0 {
                    0
                } else {
                    let tmp = cur.saturating_sub(pre);

                    if tmp.is_negative() { 0 } else { tmp }
                };

                (usage as u128 / (interval.as_nanos() / 100)) as i64
            })
        };

        Self {
            vram_usage: self.vram_usage,
            gtt_usage: self.gtt_usage,
            cpu_accessible_usage: self.cpu_accessible_usage,
            gfx,
            compute,
            dma,
            dec,
            enc,
            uvd_enc,
            vcn_jpeg: _vcn_jpeg,
        }
    }
}

pub fn get_fds(pid: i32, target_device: &str) -> Vec<i32> {
    let mut fds: Vec<i32> = Vec::new();

    let fd_path = format!("/proc/{pid}/fd/");

    let Ok(fd_list) = fs::read_dir(&fd_path) else { return fds; };

    for fd_link in fd_list {
        let Ok(dir_entry) = fd_link else { continue; };
        let dir_entry = dir_entry.path();
        let Ok(link) = fs::read_link(&dir_entry) else { continue; };

        if link.starts_with(target_device) {
            let fd_num: i32 = dir_entry.to_str().unwrap().trim_start_matches(&fd_path).parse().unwrap();
            fds.push(fd_num);
        }
    }

    fds
}

pub fn update_index(vec_info: &mut Vec<ProcInfo>, target_device: &str) {
    vec_info.clear();

    for p in procfs::process::all_processes().unwrap() {
        let prc = p.unwrap();
        let pid = prc.pid();
        let name = prc.status().unwrap().name;

        if name == env!("CARGO_PKG_NAME") { continue; }

        let fds = get_fds(pid, target_device);

        if !fds.is_empty() {
            vec_info.push(ProcInfo {
                pid,
                name,
                fds,
            });
        }
    }
}
