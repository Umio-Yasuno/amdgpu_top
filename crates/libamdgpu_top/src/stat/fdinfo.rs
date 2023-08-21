use std::fs;
use std::io::Read;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use crate::DevicePath;

/// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_fdinfo.c

#[derive(Debug, Default, Clone)]
pub struct ProcInfo {
    pub pid: i32,
    pub name: String,
    pub fds: Vec<i32>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd)]
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

impl std::ops::AddAssign for FdInfoUsage {
    fn add_assign(&mut self, other: Self) {
        self.vram_usage += other.vram_usage;
        self.gtt_usage += other.gtt_usage;
        self.cpu_accessible_usage += other.cpu_accessible_usage;
        self.gfx += other.gfx;
        self.compute += other.compute;
        self.dma += other.dma;
        self.dec += other.dec;
        self.enc += other.enc;
        self.uvd_enc += other.uvd_enc;
        self.vcn_jpeg += other.vcn_jpeg;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct ProcUsage {
    pub pid: i32,
    pub name: String,
    pub usage: FdInfoUsage,
    pub cpu_usage: i64, // %
}

#[derive(Clone, Default)]
pub struct FdInfoStat {
    pid_map: HashMap<i32, FdInfoUsage>,
    pub drm_client_ids: HashSet<usize>,
    pub proc_usage: Vec<ProcUsage>,
    pub interval: Duration,
    pub uptime: f64,
}

impl FdInfoStat {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            uptime: Self::get_uptime().unwrap_or(0.0),
            ..Default::default()
        }
    }

    fn get_uptime() -> Option<f64> {
        let s = std::fs::read_to_string("/proc/uptime").ok()?;
        let pos = s.find(" ")?;

        s[..pos].parse::<f64>().ok()
    }

    pub fn get_cpu_usage(&self, pid: i32, name: &str) -> f64 {
        const OFFSET: usize = 3;
        let Ok(s) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else { return 0.0 };
        let len = format!("{pid} ({name}) ").len();
        let split: Vec<&str> = s[len..].split(" ").collect();

        // ref: https://man7.org/linux/man-pages/man5/proc.5.html
        let [utime, stime, starttime] = [
            split.get(14-OFFSET),
            split.get(15-OFFSET),
            split.get(22-OFFSET),
        ].map(|t| t.and_then(|tt|
                tt.parse::<f64>().ok()
            ).unwrap_or(0.0)
        );

        // ref: https://stackoverflow.com/questions/16726779/how-do-i-get-the-total-cpu-usage-of-an-application-from-proc-pid-stat
        let total_time = utime + stime; // tick + tick
        let seconds = self.uptime - (starttime / 100.0); // sec - (tick / Hertz)
        let cpu_usage = 100.0 * ((total_time / 100.0) / seconds); // 100.0 * ((tick / Hertz) / sec)

        cpu_usage
    }

    pub fn get_proc_usage(&mut self, proc_info: &ProcInfo) {
        let pid = proc_info.pid;
        let name = &proc_info.name;
        let mut stat = FdInfoUsage::default();
        let mut buf = String::new();

        for fd in &proc_info.fds {
            buf.clear();
            let path = format!("/proc/{pid}/fdinfo/{fd}");
            let Ok(mut f) = fs::File::open(&path) else { continue };
            if f.read_to_string(&mut buf).is_err() { continue }

            let mut lines = buf.lines().skip_while(|l| !l.starts_with("drm-client-id"));
            if let Some(id) = lines.next().and_then(|l| FdInfoUsage::id_parse(l)) {
                if !self.drm_client_ids.insert(id) { continue }
            } else {
                continue;
            }

            'fdinfo: for l in lines {
                if l.starts_with("drm-memory") {
                    stat.mem_usage_parse(l);
                    continue 'fdinfo;
                }
                if l.starts_with("drm-engine") {
                    stat.engine_parse(l);
                    continue 'fdinfo;
                }
            }
        }

        let diff = if let Some(pre_stat) = self.pid_map.get_mut(&pid) {
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
        };

        let cpu_usage = self.get_cpu_usage(pid, &name);

        self.proc_usage.push(ProcUsage {
            pid,
            name: name.to_string(),
            usage: diff,
            cpu_usage: cpu_usage as i64,
        });
    }

    pub fn get_all_proc_usage(&mut self, proc_index: &[ProcInfo]) {
        self.proc_usage.clear();
        self.drm_client_ids.clear();
        if let Some(u) = Self::get_uptime() {
            self.uptime = u;
        }
        for pu in proc_index {
            self.get_proc_usage(pu);
        }
    }

    pub fn fold_fdinfo_usage(&self) -> FdInfoUsage {
        let mut fold = FdInfoUsage::default();

        for pu in &self.proc_usage {
            fold += pu.usage;
        }

        fold
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum FdInfoSortType {
    PID,
    #[default]
    VRAM,
    GTT,
    CPU,
    GFX,
    Compute,
    DMA,
    Decode,
    Encode,
    MediaEngine,
}

pub fn sort_proc_usage(proc_usage: &mut [ProcUsage], sort: &FdInfoSortType, reverse: bool) {
    proc_usage.sort_by(|a, b|
        match (sort, reverse) {
            (FdInfoSortType::PID, false) => b.pid.cmp(&a.pid),
            (FdInfoSortType::PID, true) => a.pid.cmp(&b.pid),
            (FdInfoSortType::VRAM, false) => b.usage.vram_usage.cmp(&a.usage.vram_usage),
            (FdInfoSortType::VRAM, true) => a.usage.vram_usage.cmp(&b.usage.vram_usage),
            (FdInfoSortType::GTT, false) => b.usage.gtt_usage.cmp(&a.usage.gtt_usage),
            (FdInfoSortType::GTT, true) => a.usage.gtt_usage.cmp(&b.usage.gtt_usage),
            (FdInfoSortType::CPU, false) => b.cpu_usage.cmp(&a.cpu_usage),
            (FdInfoSortType::CPU, true) => a.cpu_usage.cmp(&b.cpu_usage),
            (FdInfoSortType::GFX, false) => b.usage.gfx.cmp(&a.usage.gfx),
            (FdInfoSortType::GFX, true) => a.usage.gfx.cmp(&b.usage.gfx),
            (FdInfoSortType::Compute, false) => b.usage.gfx.cmp(&a.usage.compute),
            (FdInfoSortType::Compute, true) => a.usage.gfx.cmp(&b.usage.compute),
            (FdInfoSortType::DMA, false) => b.usage.gfx.cmp(&a.usage.dma),
            (FdInfoSortType::DMA, true) => a.usage.gfx.cmp(&b.usage.dma),
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

impl FdInfoUsage {
    pub fn id_parse(s: &str) -> Option<usize> {
        const LEN: usize = "drm-client-id:\t".len();
        s[LEN..].parse().ok()
    }

    pub fn mem_usage_parse(&mut self, s: &str) {
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

    pub fn engine_parse(&mut self, s: &str) {
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

                    if tmp.is_negative() { 0 } else { tmp * 100 }
                } as u128;

                usage.checked_div(interval.as_nanos()).unwrap_or(0) as i64
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

fn get_fds(pid: i32, device_path: &DevicePath) -> Vec<i32> {
    let Ok(fd_list) = fs::read_dir(format!("/proc/{pid}/fd/")) else { return Vec::new() };

    fd_list.filter_map(|fd_link| {
        let dir_entry = fd_link.map(|fd_link| fd_link.path()).ok()?;
        let link = fs::read_link(&dir_entry).ok()?;

        // e.g. "/dev/dri/renderD128" or "/dev/dri/card0"
        if [&device_path.render, &device_path.card].into_iter().any(|path| link.starts_with(path)) {
            dir_entry.file_name()?.to_str()?.parse::<i32>().ok()
        } else {
            None
        }
    }).collect()
}

pub fn get_all_processes() -> Vec<i32> {
    const SYSTEMD_CMDLINE: &[&str] = &[ "/lib/systemd", "/usr/lib/systemd" ];

    let Ok(proc_dir) = fs::read_dir("/proc") else { return Vec::new() };

    proc_dir.filter_map(|dir_entry| {
        let dir_entry = dir_entry.ok()?;
        let metadata = dir_entry.metadata().ok()?;

        if !metadata.is_dir() { return None }

        let pid = dir_entry.file_name().to_str()?.parse::<i32>().ok()?;

        if pid == 1 { return None } // init process, systemd

        // filter systemd processes from fdinfo target
        // gnome-shell share the AMDGPU driver context with systemd processes
        {
            let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline")).ok()?;
            if SYSTEMD_CMDLINE.iter().any(|path| cmdline.starts_with(path)) {
                return None;
            }
        }

        Some(pid)
    }).collect()
}

pub fn update_index_by_all_proc(
    vec_info: &mut Vec<ProcInfo>,
    device_path: &DevicePath,
    all_proc: &[i32],
) {
    vec_info.clear();

    for p in all_proc {
        let pid = *p;
        let fds = get_fds(pid, device_path);

        if fds.is_empty() { continue }

        // Maximum 16 characters
        // https://www.kernel.org/doc/html/latest/filesystems/proc.html#proc-pid-comm-proc-pid-task-tid-comm
        let Ok(mut name) = fs::read_to_string(format!("/proc/{pid}/comm")) else { continue };
        name.pop(); // trim '\n'

        vec_info.push(ProcInfo { pid, name, fds });
    }
}

pub fn update_index(vec_info: &mut Vec<ProcInfo>, device_path: &DevicePath) {
    update_index_by_all_proc(vec_info, device_path, &get_all_processes());
}
