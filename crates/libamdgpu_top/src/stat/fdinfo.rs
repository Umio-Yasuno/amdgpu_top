use std::fs;
use std::io::Read;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
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
    pub system_cpu_usage: u64, // KiB, from Linux Kernel v6.4
    pub gfx: i64,
    pub compute: i64,
    pub dma: i64,
    pub dec: i64,
    pub enc: i64,
    pub uvd_enc: i64,
    pub vcn_jpeg: i64,
    pub media: i64,
}

impl std::ops::AddAssign for FdInfoUsage {
    fn add_assign(&mut self, other: Self) {
        self.vram_usage += other.vram_usage;
        self.gtt_usage += other.gtt_usage;
        self.system_cpu_usage += other.system_cpu_usage;
        self.gfx += other.gfx;
        self.compute += other.compute;
        self.dma += other.dma;
        self.dec += other.dec;
        self.enc += other.enc;
        self.uvd_enc += other.uvd_enc;
        self.vcn_jpeg += other.vcn_jpeg;
        self.media += other.media;
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
    pub pid_map: HashMap<i32, FdInfoUsage>,
    pub drm_client_ids: HashSet<usize>,
    pub proc_usage: Vec<ProcUsage>,
    pub interval: Duration,
    pub cpu_time_map: HashMap<i32, f32>, // sec
    pub has_vcn: bool,
    pub has_vcn_unified: bool,
}

impl FdInfoStat {
/*
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            ..Default::default()
        }
    }
*/
    pub fn get_cpu_usage(&mut self, pid: i32, name: &str) -> f32 {
        const OFFSET: usize = 3;
        const HZ: f32 = 100.0;
        let Ok(s) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else { return 0.0 };
        // for process names with spaces
        let len = format!("{pid} ({name}) ").len();

        let split: Vec<&str> = if let Some(s) = s.get(len..) {
            s.split(' ').collect()
        } else {
            return 0.0;
        };

        // ref: https://man7.org/linux/man-pages/man5/proc.5.html
        let [utime, stime] = [
            split.get(14-OFFSET),
            split.get(15-OFFSET),
        ].map(|t| t.and_then(|tt|
                tt.parse::<f32>().ok()
            ).unwrap_or(0.0)
        );

        // ref: https://stackoverflow.com/questions/16726779/how-do-i-get-the-total-cpu-usage-of-an-application-from-proc-pid-stat
        let total_time = (utime + stime) / HZ; // sec = (tick + tick) / HZ

        if let Some(pre_cpu_time) = self.cpu_time_map.get_mut(&pid) {
            let tmp = total_time - *pre_cpu_time;
            *pre_cpu_time = total_time;

            tmp * 100.0 / self.interval.as_secs_f32()
        } else {
            self.cpu_time_map.insert(pid, total_time);

            0.0
        }
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
            if let Some(id) = lines.next().and_then(FdInfoUsage::id_parse) {
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
            let tmp = stat.calc_usage(pre_stat, &self.interval, self.has_vcn, self.has_vcn_unified);
            *pre_stat = stat;

            tmp
        } else {
            let [vram_usage, gtt_usage, system_cpu_usage] = [
                stat.vram_usage,
                stat.gtt_usage,
                stat.system_cpu_usage,
            ];

            self.pid_map.insert(pid, stat);

            FdInfoUsage {
                vram_usage,
                gtt_usage,
                system_cpu_usage,
                ..Default::default()
            }
        };

        let cpu_usage = self.get_cpu_usage(pid, name);

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

    pub fn sort_proc_usage(&mut self, sort: FdInfoSortType, reverse: bool) {
        self.proc_usage.sort_by(|a, b|
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
                (FdInfoSortType::MediaEngine, false) => b.usage.media.cmp(&a.usage.media),
                (FdInfoSortType::MediaEngine, true) => a.usage.media.cmp(&b.usage.media),
            }
        );
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
        s.get(LEN..)?.parse().ok()
    }

    pub fn mem_usage_parse(&mut self, s: &str) {
        const PRE: usize = "drm-memory-xxxx:\t".len(); // "vram:" or "gtt: " or "cpu: "
        const KIB: usize = " KiB".len();
        let len = s.len();

        const MEM_TYPE: std::ops::Range<usize> = {
            const PRE_LEN: usize = "drm-memory-".len();

            PRE_LEN..(PRE_LEN+5)
        };

        let usage = s.get(PRE..len-KIB).and_then(|s| s.parse().ok()).unwrap_or(0);

        match &s[MEM_TYPE] {
            "vram:" => self.vram_usage += usage,
            "gtt: " => self.gtt_usage += usage,
            "cpu: " => self.system_cpu_usage += usage, // from Linux Kernel v6.4
            _ => {},
        };
    }

    pub fn engine_parse(&mut self, s: &str) {
        const PRE: usize = "drm-engine-".len();
        const NS: usize = " ns".len();
        let Some(pos) = s.find('\t') else { return };

        let ns: i64 = {
            let len = s.len();
            s.get(pos+1..len-NS).and_then(|s| s.parse().ok()).unwrap_or(0)
        };
        let Some(s) = s.get(PRE..pos) else { return };

        match s {
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

    pub fn calc_usage(
        &self,
        pre_stat: &Self,
        interval: &Duration,
        has_vcn: bool,
        has_vcn_unified: bool,
    ) -> Self {
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

        /*
            From VCN4, the encoding queue and decoding queue have been unified.
            The AMDGPU driver handles both decoding and encoding as contexts for the encoding engine.
        */
        let media = if has_vcn_unified {
            (vcn_jpeg + enc) / 2
        } else if has_vcn {
            (dec + vcn_jpeg + enc) / 3
        } else {
            (dec + vcn_jpeg + enc + uvd_enc) / 4
        };

        Self {
            vram_usage: self.vram_usage,
            gtt_usage: self.gtt_usage,
            system_cpu_usage: self.system_cpu_usage,
            gfx,
            compute,
            dma,
            dec,
            enc,
            uvd_enc,
            vcn_jpeg,
            media,
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

pub fn spawn_update_index_thread(
    t_index: Vec<(DevicePath, Arc<Mutex<Vec<ProcInfo>>>)>,
    interval: u64,
) {
    let mut buf_index: Vec<ProcInfo> = Vec::new();
    let interval = Duration::from_secs(interval);

    std::thread::spawn(move || loop {
        std::thread::sleep(interval);

        let all_proc = get_all_processes();

        for (device_path, index) in &t_index {
            update_index_by_all_proc(&mut buf_index, device_path, &all_proc);

            let lock = index.lock();
            if let Ok(mut index) = lock {
                *index = buf_index.clone();
            }
        }
    });
}
