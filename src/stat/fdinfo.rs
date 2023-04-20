use std::fs;
use std::io::Read;
use std::fmt::{self, Write};
use super::{Text, Opt};
// use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use serde_json::{json, Map, Value};
use crate::DevicePath;

/// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_fdinfo.c

const PROC_NAME_LEN: usize = 16;

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

impl ProcInfo {
    pub fn from_pid(pid: i32, device_path: &DevicePath) -> Self {
        let mut name = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap();
        name.pop(); // trim '\n'

        Self {
            pid,
            name,
            fds: get_fds(pid, device_path),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct FdInfoUsage {
    // client_id: usize,
    pub vram_usage: u64, // KiB
    pub gtt_usage: u64, // KiB
    pub cpu_accessible_usage: u64, // KiB
    pub gfx: i64,
    pub compute: i64,
    pub dma: i64,
    pub dec: i64,
    pub enc: i64,
    pub uvd_enc: i64,
    pub vcn_jpeg: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct ProcUsage {
    pub pid: i32,
    pub name: String,
    pub usage: FdInfoUsage,
}

#[derive(Clone, Default)]
pub struct FdInfoView {
    pid_map: HashMap<i32, FdInfoUsage>,
    pub drm_client_ids: HashSet<usize>,
    pub proc_usage: Vec<ProcUsage>,
    pub interval: Duration,
    pub text: Text,
}

pub fn sort_proc_usage(proc_usage: &mut [ProcUsage], sort: &FdInfoSortType, reverse: bool) {
    proc_usage.sort_by(|a, b|
        match (sort, reverse) {
            (FdInfoSortType::PID, false) => b.pid.cmp(&a.pid),
            (FdInfoSortType::PID, true) => a.pid.cmp(&b.pid),
            (FdInfoSortType::VRAM, false) => b.usage.vram_usage.cmp(&a.usage.vram_usage),
            (FdInfoSortType::VRAM, true) => a.usage.vram_usage.cmp(&b.usage.vram_usage),
            (FdInfoSortType::GFX, false) => b.usage.gfx.cmp(&a.usage.gfx),
            (FdInfoSortType::GFX, true) => a.usage.gfx.cmp(&b.usage.gfx),
            (FdInfoSortType::Decode, false) =>
                (b.usage.dec + b.usage.vcn_jpeg).cmp(&(a.usage.dec + a.usage.vcn_jpeg)),
            (FdInfoSortType::Decode, true) =>
                (a.usage.dec + a.usage.vcn_jpeg).cmp(&(b.usage.dec + b.usage.vcn_jpeg)),
            (FdInfoSortType::Encode, false) =>
                (b.usage.enc + b.usage.uvd_enc).cmp(&(a.usage.enc + a.usage.uvd_enc)),
            (FdInfoSortType::Encode, true) =>
                (a.usage.enc + a.usage.uvd_enc).cmp(&(b.usage.enc + b.usage.uvd_enc)),
            (FdInfoSortType::MediaEngine, false) =>
                (b.usage.dec + b.usage.vcn_jpeg + b.usage.enc + b.usage.uvd_enc)
                    .cmp(&(a.usage.dec + a.usage.vcn_jpeg + a.usage.enc + a.usage.uvd_enc)),
            (FdInfoSortType::MediaEngine, true) =>
                (a.usage.dec + a.usage.vcn_jpeg + a.usage.enc + a.usage.uvd_enc)
                    .cmp(&(b.usage.dec + b.usage.vcn_jpeg + b.usage.enc + b.usage.uvd_enc)),
        }
    );
}

#[derive(Clone, Debug)]
pub enum FdInfoSortType {
    PID,
    VRAM,
    GFX,
    Decode,
    Encode,
    MediaEngine,
}

impl FdInfoView {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            ..Default::default()
        }
    }

    pub fn print(
        &mut self,
        slice_proc_info: &[ProcInfo],
        sort: &FdInfoSortType,
        reverse: bool
    ) -> Result<(), fmt::Error> {
        self.text.clear();
        self.proc_usage.clear();
        self.drm_client_ids.clear();

        writeln!(
            self.text.buf,
            " {pad:27} | {VRAM_LABEL:^8} | {GFX_LABEL} | {COMPUTE_LABEL} | {DMA_LABEL} | {DEC_LABEL} | {ENC_LABEL} |",
            pad = "",
        )?;

        for proc_info in slice_proc_info {
            self.get_proc_usage(proc_info);
        }

        sort_proc_usage(&mut self.proc_usage, sort, reverse);

        self.print_usage()?;

        Ok(())
    }

    pub fn print_usage(&mut self) -> Result<(), fmt::Error> {
        for pu in &self.proc_usage {
            let utf16_count = pu.name.encode_utf16().count();
            let name_len = if pu.name.len() != utf16_count {
                PROC_NAME_LEN - utf16_count
            } else {
                PROC_NAME_LEN
            };
            write!(
                self.text.buf,
                " {name:name_len$} ({pid:>8}) | {vram:>5} MiB|",
                name = pu.name,
                pid = pu.pid,
                vram = pu.usage.vram_usage >> 10,
            )?;
            let dec_usage = pu.usage.dec + pu.usage.vcn_jpeg;
            let enc_usage = pu.usage.enc + pu.usage.uvd_enc;
            for (usage, label_len) in [
                (pu.usage.gfx, GFX_LABEL.len()),
                (pu.usage.compute, COMPUTE_LABEL.len()),
                (pu.usage.dma, DMA_LABEL.len()),
                (dec_usage, DEC_LABEL.len()), // UVD/VCN/VCN_JPEG
                (enc_usage, ENC_LABEL.len()), // UVD/VCN
            ] {
                write!(self.text.buf, " {usage:>label_len$}%|")?;
            }
            writeln!(self.text.buf)?;
        }

        Ok(())
    }

    pub fn get_proc_usage(&mut self, proc_info: &ProcInfo) {
        let pid = proc_info.pid;
        let name = &proc_info.name;
        let mut stat = FdInfoUsage::default();
        let mut buf = String::new();

        'fds: for fd in &proc_info.fds {
            let path = format!("/proc/{pid}/fdinfo/{fd}");
            let Ok(mut f) = fs::File::open(&path) else { continue };
            if f.read_to_string(&mut buf).is_err() { continue }
            let mut lines = buf.lines();

            'fdinfo: loop {
                let Some(l) = lines.next() else { break 'fdinfo; };

                if l.starts_with("drm-client-id") {
                    let id = FdInfoUsage::id_parse(l);
                    if !self.drm_client_ids.insert(id) { continue 'fds; }
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

    pub fn json_value(&self) -> Value {
        let Some(pu) = self.proc_usage.get(0) else { return Value::Null };
        let mut m = Map::new();

        for (usage, label) in [
            (pu.usage.vram_usage >> 10, "VRAM Usage"),
            (pu.usage.gtt_usage >> 10, "GTT Usage"),
        ] {
            m.insert(
                label.to_string(),
                json!({
                    "value": usage,
                    "unit": "MiB",
                }),
            );
        }

        let dec_usage = pu.usage.dec + pu.usage.vcn_jpeg;
        let enc_usage = pu.usage.enc + pu.usage.uvd_enc;
        for (usage, label) in [
            (pu.usage.gfx, GFX_LABEL),
            (pu.usage.compute, COMPUTE_LABEL),
            (pu.usage.dma, DMA_LABEL),
            (dec_usage, DEC_LABEL), // UVD/VCN
            (enc_usage, ENC_LABEL),
        ] {
            m.insert(
                label.to_string(),
                json!({
                    "value": usage,
                    "unit": "%",
                }),
            );
        }

        m.into()
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

    pub fn cb_sort_by_media(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::MediaEngine;
        }
    }
}

impl FdInfoUsage {
    fn id_parse(s: &str) -> usize {
        const LEN: usize = "drm-client-id:\t".len();
        s[LEN..].parse().unwrap()
    }

    fn mem_usage_parse(&mut self, s: &str) {
        const PRE: usize = "drm-memory-xxxx:\t".len(); // "vram:" or "gtt: " or "cpu: "
        const KIB: usize = " KiB".len();
        let len = s.len();

        const MEM_TYPE: std::ops::Range<usize> = {
            const PRE_LEN: usize = "drm-memory-".len();

            PRE_LEN..(PRE_LEN+5)
        };

        let usage = s[PRE..(len-KIB)].parse().unwrap_or(0);

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
        let Some(pos) = s.find('\t') else { return };

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
        let [gfx, compute, dma, dec, enc, uvd_enc, vcn_jpeg] = {
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
            vcn_jpeg,
        }
    }
}

pub fn get_self_pid() -> Option<i32> {
    let link = std::fs::read_link("/proc/self").ok()?;
    let path_str = link.to_str()?;

    path_str.parse::<i32>().ok()
}

use std::path::PathBuf;

fn get_fds(pid: i32, device_path: &DevicePath) -> Vec<i32> {
    let mut fds: Vec<i32> = Vec::new();
    let fd_path = format!("/proc/{pid}/fd/");
    let Ok(fd_list) = fs::read_dir(&fd_path) else { return fds };

    for fd_link in fd_list {
        let Ok(dir_entry) = fd_link.map(|fd_link| fd_link.path()) else { continue };
        let Ok(link) = fs::read_link(&dir_entry) else { continue };

        // e.g. "/dev/dri/renderD128" or "/dev/dri/card0"
        if [&device_path.render, &device_path.card].into_iter().any(|path| link.starts_with(path)) {
            let Some(fd_num) = dir_entry.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<i32>().ok()) else { continue };
            fds.push(fd_num);
        }
    }

    fds
}

fn get_all_processes() -> Vec<i32> {
    let mut pids: Vec<i32> = Vec::new();
    let Ok(proc_dir) = fs::read_dir("/proc") else { return pids };

    for dir_entry in proc_dir.into_iter().flatten() {
        let Ok(metadata) = dir_entry.metadata() else { continue };
        if !metadata.is_dir() { continue }

        let Some(pid) = dir_entry.file_name().to_str()
            .and_then(|name| name.parse::<i32>().ok()) else { continue };
        pids.push(pid);
    }

    pids
}

pub fn update_index(vec_info: &mut Vec<ProcInfo>, device_path: &DevicePath, self_pid: i32) {
    const SYSTEMD_CMDLINE: &[&str] = &[ "/lib/systemd", "/usr/lib/systemd" ];

    vec_info.clear();

    for p in &get_all_processes() {
        let pid = *p;
        if pid == self_pid { continue }
        if pid == 1 { continue } // init process, systemd

        let fds = get_fds(pid, device_path);

        if !fds.is_empty() {
            let base = PathBuf::from(format!("/proc/{pid}/"));
            {
                // filter systemd processes from fdinfo target
                // gnome-shell share the AMDGPU driver context with systemd processes
                let Ok(cmdline) = fs::read_to_string(base.join("cmdline")) else { continue };
                if SYSTEMD_CMDLINE.into_iter().any(|path| cmdline.starts_with(path)) {
                    continue
                }
            }
            // Maximum 16 characters
            // https://www.kernel.org/doc/html/latest/filesystems/proc.html#proc-pid-comm-proc-pid-task-tid-comm
            let Ok(mut name) = fs::read_to_string(base.join("comm")) else {
                continue
            };
            name.pop(); // trim '\n'

            vec_info.push(ProcInfo {
                pid,
                name,
                fds,
            });
        }
    }
}
