use std::fs;
use std::io::Read;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::path::Path;
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
    pub system_cpu_memory_usage: u64, // KiB, from Linux Kernel v6.4
    pub amd_visible_vram: u64, // KiB, from Linux Kernel v6.4
    pub amd_evicted_vram: u64, // KiB, from Linux Kernel v6.4
    pub amd_evicted_visible_vram: u64, // KiB, from Linux Kernel v6.4
    pub amd_requested_vram: u64, // KiB, from Linux Kernel v6.4
    pub amd_requested_gtt: u64, // KiB, from Linux Kernel v6.4
    pub amd_requested_visible_vram: u64, // KiB, from Linux Kernel v6.4
    pub gfx: i64,
    pub compute: i64,
    pub dma: i64,
    pub dec: i64,
    pub enc: i64,
    pub uvd_enc: i64,
    pub vcn_jpeg: i64,
    pub media: i64,
    pub total_dec: i64,
    pub total_enc: i64,
    pub vpe: i64,
}

impl std::ops::Add for FdInfoUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            vram_usage: self.vram_usage + other.vram_usage,
            gtt_usage: self.gtt_usage + other.gtt_usage,
            system_cpu_memory_usage: self.system_cpu_memory_usage + other.system_cpu_memory_usage,
            amd_visible_vram: self.amd_visible_vram + other.amd_visible_vram,
            amd_evicted_vram: self.amd_evicted_vram + other.amd_evicted_vram,
            amd_evicted_visible_vram: self.amd_evicted_visible_vram + other.amd_evicted_visible_vram,
            amd_requested_vram: self.amd_requested_vram + other.amd_requested_vram,
            amd_requested_gtt: self.amd_requested_gtt + other.amd_requested_gtt,
            amd_requested_visible_vram: self.amd_requested_visible_vram + other.amd_requested_visible_vram,
            gfx: self.gfx + other.gfx,
            compute: self.compute + other.compute,
            dma: self.dma + other.dma,
            dec: self.dec + other.dec,
            enc: self.enc + other.enc,
            uvd_enc: self.uvd_enc + other.uvd_enc,
            vcn_jpeg: self.vcn_jpeg + other.vcn_jpeg,
            media: self.media + other.media,
            total_dec: self.total_dec + other.total_dec,
            total_enc: self.total_enc + other.total_enc,
            vpe: self.vpe + other.vpe,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct ProcUsage {
    pub pid: i32,
    pub name: String,
    pub ids_count: usize,
    pub usage: FdInfoUsage,
    pub cpu_usage: i64, // %
    pub is_kfd_process: bool,
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
    pub has_vpe: bool,
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
        let mut ids_count = 0usize;

        for fd in &proc_info.fds {
            buf.clear();
            let path = format!("/proc/{pid}/fdinfo/{fd}");
            let Ok(mut f) = fs::File::open(&path) else { continue };
            if f.read_to_string(&mut buf).is_err() { continue }

            let mut lines = buf.lines().skip_while(|l| !l.starts_with("drm-client-id"));
            if let Some(id) = lines.next().and_then(FdInfoUsage::id_parse) {
                ids_count += 1;
                if !self.drm_client_ids.insert(id) { continue }
            } else {
                continue;
            }

            for l in lines {
                let Some(s) = l.get(0..10) else { continue };

                match s {
                    "drm-memory" => stat.mem_usage_parse(l),
                    "drm-engine" => stat.engine_parse(l),
                    "amd-memory" => stat.visible_vram_parse(l),
                    "amd-evicte" => stat.evicted_vram_parse(l),
                    "amd-reques" => stat.requested_vram_parse(l),
                    _ => {},
                }
            }
        }

        let diff = if let Some(pre_stat) = self.pid_map.get_mut(&pid) {
            let tmp = stat.calc_usage(pre_stat, &self.interval, self.has_vcn, self.has_vcn_unified);
            *pre_stat = stat;

            tmp
        } else {
            let [
                vram_usage,
                gtt_usage,
                system_cpu_memory_usage,
                amd_visible_vram,
                amd_evicted_vram,
                amd_evicted_visible_vram,
                amd_requested_vram,
                amd_requested_visible_vram,
                amd_requested_gtt,
            ] = [
                stat.vram_usage,
                stat.gtt_usage,
                stat.system_cpu_memory_usage,
                stat.amd_visible_vram,
                stat.amd_evicted_vram,
                stat.amd_evicted_visible_vram,
                stat.amd_requested_vram,
                stat.amd_requested_visible_vram,
                stat.amd_requested_gtt,
            ];

            self.pid_map.insert(pid, stat);

            FdInfoUsage {
                vram_usage,
                gtt_usage,
                system_cpu_memory_usage,
                amd_visible_vram,
                amd_evicted_vram,
                amd_evicted_visible_vram,
                amd_requested_vram,
                amd_requested_visible_vram,
                amd_requested_gtt,
                ..Default::default()
            }
        };

        let cpu_usage = self.get_cpu_usage(pid, name);
        let is_kfd_process = Path::new("/sys/class/kfd/kfd/proc/").join(pid.to_string()).exists();

        self.proc_usage.push(ProcUsage {
            pid,
            name: name.to_string(),
            ids_count,
            usage: diff,
            cpu_usage: cpu_usage as i64,
            is_kfd_process,
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
        self.proc_usage.iter().fold(FdInfoUsage::default(), |acc, pu| acc + pu.usage)
    }

    pub fn sort_proc_usage(&mut self, sort: FdInfoSortType, reverse: bool) {
        self.proc_usage.sort_by(|a, b|
            match (sort, reverse) {
                (FdInfoSortType::PID, false) => b.pid.cmp(&a.pid),
                (FdInfoSortType::PID, true) => a.pid.cmp(&b.pid),
                (FdInfoSortType::KFD, false) => b.is_kfd_process.cmp(&a.is_kfd_process),
                (FdInfoSortType::KFD, true) => a.is_kfd_process.cmp(&b.is_kfd_process),
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
                (FdInfoSortType::Decode, false) => b.usage.total_dec.cmp(&a.usage.total_dec),
                (FdInfoSortType::Decode, true) => a.usage.total_dec.cmp(&b.usage.total_dec),
                (FdInfoSortType::Encode, false) => b.usage.total_enc.cmp(&a.usage.total_enc),
                (FdInfoSortType::Encode, true) => a.usage.total_enc.cmp(&b.usage.total_enc),
                (FdInfoSortType::MediaEngine, false) => b.usage.media.cmp(&a.usage.media),
                (FdInfoSortType::MediaEngine, true) => a.usage.media.cmp(&b.usage.media),
                (FdInfoSortType::VPE, false) => b.usage.media.cmp(&a.usage.vpe),
                (FdInfoSortType::VPE, true) => a.usage.media.cmp(&b.usage.vpe),
            }
        );
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum FdInfoSortType {
    PID,
    KFD,
    #[default]
    VRAM,
    GTT,
    CPU,
    GFX,
    Compute,
    DMA, // SDMA, System DMA Engine
    Decode,
    Encode,
    MediaEngine,
    VPE, // Video Processing Engine
}

impl FdInfoUsage {
    const KIB: usize = " KiB".len();

    pub fn id_parse(s: &str) -> Option<usize> {
        const LEN: usize = "drm-client-id:\t".len();
        s.get(LEN..)?.parse().ok()
    }

    pub fn mem_usage_parse(&mut self, s: &str) {
        const PRE: usize = "drm-memory-xxxx:\t".len(); // "vram:" or "gtt: " or "cpu: "
        const KIB: usize = " KiB".len();
        const MEM_TYPE: std::ops::Range<usize> = {
            const PRE_LEN: usize = "drm-memory-".len();

            PRE_LEN..(PRE_LEN+5)
        };

        let Some(usage) = s.get(PRE..s.len()-KIB).and_then(|s| s.parse::<u64>().ok()) else { return };
        let Some(mem_type) = s.get(MEM_TYPE) else { return };

        match mem_type {
            "vram:" => self.vram_usage += usage,
            "gtt: " => self.gtt_usage += usage,
            "cpu: " => self.system_cpu_memory_usage += usage, // from Linux Kernel v6.4
            _ => {},
        };
    }

    pub fn engine_parse(&mut self, s: &str) {
        const PRE: usize = "drm-engine-".len();
        const NS: usize = " ns".len();
        let Some(pos) = s.find('\t') else { return };

        let Some(ns) = s.get(pos+1..s.len()-NS).and_then(|s| s.parse::<i64>().ok()) else { return };
        let Some(s) = s.get(PRE..pos) else { return };

        match s {
            "gfx:" => self.gfx += ns,
            "compute:" => self.compute += ns,
            "dma:" => self.dma += ns,
            "dec:" => self.dec += ns,
            "enc:" => self.enc += ns,
            "enc_1:" => self.uvd_enc += ns,
            "jpeg:" => self.vcn_jpeg += ns,
            "vpe:" => self.vpe += ns,
            _ => {},
        };
    }

    pub fn visible_vram_parse(&mut self, s: &str) {
        const PRE_LEN: usize = "amd-memory-visible-vram:\t".len();
        let Some(m) = s.get(PRE_LEN..s.len()-Self::KIB)
            .and_then(|m| m.parse::<u64>().ok()) else { return };

        self.amd_visible_vram += m;
    }

    pub fn evicted_vram_parse(&mut self, s: &str) {
        enum EvictedVramType {
            Vram,
            VisibleVram,
        }

        impl EvictedVramType {
            const PRE_LEN: usize = "amd-evicted-".len();
            const VRAM_TYPE: std::ops::Range<usize> = Self::PRE_LEN..(Self::PRE_LEN+4);

            fn from_line(s: &str) -> Option<Self> {
                match s.get(Self::VRAM_TYPE)? {
                    "vram" => Some(Self::Vram),
                    "visi" => Some(Self::VisibleVram),
                    _ => None
                }
            }

            const fn vram_pos(&self) -> usize {
                match self {
                    Self::Vram => Self::PRE_LEN + "vram:\t".len(),
                    Self::VisibleVram => Self::PRE_LEN + "visible-vram:\t".len(),
                }
            }
        }

        let Some(vram_type) = EvictedVramType::from_line(s) else { return };
        let Some(m) = s.get(vram_type.vram_pos()..s.len()-Self::KIB)
            .and_then(|m| m.parse::<u64>().ok()) else { return };

        match vram_type {
            EvictedVramType::Vram => self.amd_evicted_vram += m,
            EvictedVramType::VisibleVram => self.amd_evicted_visible_vram += m,
        }
    }

    pub fn requested_vram_parse(&mut self, s: &str) {
        enum RequestedVramType {
            Vram,
            VisibleVram,
            Gtt,
        }

        impl RequestedVramType {
            const PRE_LEN: usize = "amd-requested-".len();
            const VRAM_TYPE: std::ops::Range<usize> = Self::PRE_LEN..(Self::PRE_LEN+4);

            fn from_line(s: &str) -> Option<Self> {
                match s.get(Self::VRAM_TYPE)? {
                    "vram" => Some(Self::Vram),
                    "visi" => Some(Self::VisibleVram),
                    "gtt:" => Some(Self::Gtt),
                    _ => None
                }
            }

            const fn vram_pos(&self) -> usize {
                match self {
                    Self::Vram => Self::PRE_LEN + "vram:\t".len(),
                    Self::VisibleVram => Self::PRE_LEN + "visible-vram:\t".len(),
                    Self::Gtt => Self::PRE_LEN + "gtt:\t".len(),
                }
            }
        }

        let Some(vram_type) = RequestedVramType::from_line(s) else { return };
        let Some(m) = s.get(vram_type.vram_pos()..s.len()-Self::KIB)
            .and_then(|m| m.parse::<u64>().ok()) else { return };

        match vram_type {
            RequestedVramType::Vram => self.amd_requested_vram += m,
            RequestedVramType::VisibleVram => self.amd_requested_visible_vram += m,
            RequestedVramType::Gtt => self.amd_requested_gtt += m,
        }
    }

    pub fn calc_usage(
        &self,
        pre_stat: &Self,
        interval: &Duration,
        has_vcn: bool,
        has_vcn_unified: bool,
    ) -> Self {
        let [gfx, compute, dma, dec, enc, uvd_enc, vcn_jpeg, vpe] = {
            [
                (pre_stat.gfx, self.gfx),
                (pre_stat.compute, self.compute),
                (pre_stat.dma, self.dma),
                (pre_stat.dec, self.dec),
                (pre_stat.enc, self.enc),
                (pre_stat.uvd_enc, self.uvd_enc),
                (pre_stat.vcn_jpeg, self.vcn_jpeg),
                (pre_stat.vpe, self.vpe),
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
        let [total_dec, total_enc, media] = if has_vcn_unified {
            let media = (vcn_jpeg + enc) / 2;

            [0, 0, media]
        } else if has_vcn {
            let total_dec = (dec + vcn_jpeg) / 2;
            let media = (dec + vcn_jpeg + enc) / 3;

            [total_dec, enc, media]
        } else {
            let total_enc = (enc + uvd_enc) / 2;
            let media = (dec + enc + uvd_enc) / 3;

            [dec, total_enc, media]
        };

        Self {
            vram_usage: self.vram_usage,
            gtt_usage: self.gtt_usage,
            system_cpu_memory_usage: self.system_cpu_memory_usage,
            amd_visible_vram: self.amd_visible_vram,
            amd_evicted_vram: self.amd_evicted_vram,
            amd_evicted_visible_vram: self.amd_evicted_visible_vram,
            amd_requested_vram: self.amd_requested_vram,
            amd_requested_gtt: self.amd_requested_gtt,
            amd_requested_visible_vram: self.amd_requested_visible_vram,
            gfx,
            compute,
            dma,
            dec,
            enc,
            uvd_enc,
            vcn_jpeg,
            media,
            total_dec,
            total_enc,
            vpe,
        }
    }
}

fn get_fds<T: AsRef<Path>>(pid: i32, device_path: &[T]) -> Vec<i32> {
    let Ok(fd_list) = fs::read_dir(format!("/proc/{pid}/fd/")) else { return Vec::new() };

    fd_list.filter_map(|fd_link| {
        let dir_entry = fd_link.map(|fd_link| fd_link.path()).ok()?;
        let link = fs::read_link(&dir_entry).ok()?;

        // e.g. "/dev/dri/renderD128" or "/dev/dri/card0"
        if device_path
            .into_iter()
            .any(|path| link.starts_with(path))
        {
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

pub fn update_index_by_all_proc<T: AsRef<Path>>(
    vec_info: &mut Vec<ProcInfo>,
    device_path: &[T],
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
    update_index_by_all_proc(
        vec_info,
        &[&device_path.render, &device_path.card],
        &get_all_processes(),
    );
}

pub fn spawn_update_index_thread(
    device_paths: Vec<DevicePath>,
    interval: u64,
) {
    let mut buf_index: Vec<ProcInfo> = Vec::new();
    let interval = Duration::from_secs(interval);

    std::thread::spawn(move || loop {
        let all_proc = get_all_processes();

        for device_path in &device_paths {
            update_index_by_all_proc(
                &mut buf_index,
                &[&device_path.render, &device_path.card],
                &all_proc,
            );

            let lock = device_path.arc_proc_index.lock();
            if let Ok(mut index) = lock {
                index.clone_from(&buf_index);
            }
        }

        std::thread::sleep(interval);
    });
}
