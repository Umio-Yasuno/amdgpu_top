use std::fs;
use std::io::Read;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::path::Path;
use super::ProcInfo;
use crate::stat;

const KFD_PROC_PATH: &str = "/sys/class/kfd/kfd/proc/";

// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_fdinfo.c

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct FdInfoUsage {
    // client_id: usize,
    pub cpu: i64, // %
    pub vram_usage: u64, // KiB
    pub gtt_usage: u64, // KiB
    pub system_cpu_memory_usage: u64, // KiB, from Linux Kernel v6.4
    pub amd_evicted_vram: u64, // KiB, from Linux Kernel v6.4
    pub amd_requested_vram: u64, // KiB, from Linux Kernel v6.4
    pub amd_requested_gtt: u64, // KiB, from Linux Kernel v6.4
    pub gfx: i64, // ns, %
    pub compute: i64, // ns, %
    pub dma: i64, // ns, %
    pub dec: i64, // ns, %
    pub enc: i64, // ns, %
    pub uvd_enc: i64, // ns, %
    pub vcn_jpeg: i64, // ns, %
    pub media: i64, // ns, %
    pub total_dec: i64, // ns, %
    pub total_enc: i64, // ns, %
    pub vpe: i64, // ns, %
    pub vcn_unified: i64, // ns, %, dec+enc
}

impl std::ops::Add for FdInfoUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            cpu: self.cpu + other.cpu,
            vram_usage: self.vram_usage + other.vram_usage,
            gtt_usage: self.gtt_usage + other.gtt_usage,
            system_cpu_memory_usage: self.system_cpu_memory_usage + other.system_cpu_memory_usage,
            amd_evicted_vram: self.amd_evicted_vram + other.amd_evicted_vram,
            amd_requested_vram: self.amd_requested_vram + other.amd_requested_vram,
            amd_requested_gtt: self.amd_requested_gtt + other.amd_requested_gtt,
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
            vcn_unified: self.vcn_unified + other.vcn_unified,
        }
    }
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

    pub fn evicted_vram_parse(&mut self, s: &str) {
        enum EvictedVramType {
            Vram,
        }

        impl EvictedVramType {
            const PRE_LEN: usize = "amd-evicted-".len();
            const VRAM_TYPE: std::ops::Range<usize> = Self::PRE_LEN..(Self::PRE_LEN+4);

            fn from_line(s: &str) -> Option<Self> {
                match s.get(Self::VRAM_TYPE)? {
                    "vram" => Some(Self::Vram),
                    _ => None
                }
            }

            const fn vram_pos(&self) -> usize {
                match self {
                    Self::Vram => Self::PRE_LEN + "vram:\t".len(),
                }
            }
        }

        let Some(vram_type) = EvictedVramType::from_line(s) else { return };
        let Some(m) = s.get(vram_type.vram_pos()..s.len()-Self::KIB)
            .and_then(|m| m.parse::<u64>().ok()) else { return };

        match vram_type {
            EvictedVramType::Vram => self.amd_evicted_vram += m,
        }
    }

    pub fn requested_vram_parse(&mut self, s: &str) {
        enum RequestedVramType {
            Vram,
            Gtt,
        }

        impl RequestedVramType {
            const PRE_LEN: usize = "amd-requested-".len();
            const VRAM_TYPE: std::ops::Range<usize> = Self::PRE_LEN..(Self::PRE_LEN+4);

            fn from_line(s: &str) -> Option<Self> {
                match s.get(Self::VRAM_TYPE)? {
                    "vram" => Some(Self::Vram),
                    "gtt:" => Some(Self::Gtt),
                    _ => None
                }
            }

            const fn vram_pos(&self) -> usize {
                match self {
                    Self::Vram => Self::PRE_LEN + "vram:\t".len(),
                    Self::Gtt => Self::PRE_LEN + "gtt:\t".len(),
                }
            }
        }

        let Some(vram_type) = RequestedVramType::from_line(s) else { return };
        let Some(m) = s.get(vram_type.vram_pos()..s.len()-Self::KIB)
            .and_then(|m| m.parse::<u64>().ok()) else { return };

        match vram_type {
            RequestedVramType::Vram => self.amd_requested_vram += m,
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
            .map(|(pre, cur)| stat::diff_usage(pre, cur, interval))
        };

        /*
            From VCN4, the encoding queue and decoding queue have been unified.
            The AMDGPU driver handles both decoding and encoding as contexts for the encoding engine.
        */
        let [total_dec, total_enc, media, vcn_unified] = if has_vcn_unified {
            let media = (vcn_jpeg + enc) / 2;

            [0, 0, media, dec+enc]
        } else if has_vcn {
            let total_dec = (dec + vcn_jpeg) / 2;
            let media = (dec + vcn_jpeg + enc) / 3;

            [total_dec, enc, media, 0]
        } else {
            let total_enc = (enc + uvd_enc) / 2;
            let media = (dec + enc + uvd_enc) / 3;

            [dec, total_enc, media, 0]
        };

        Self {
            cpu: 0,
            vram_usage: self.vram_usage,
            gtt_usage: self.gtt_usage,
            system_cpu_memory_usage: self.system_cpu_memory_usage,
            amd_evicted_vram: self.amd_evicted_vram,
            amd_requested_vram: self.amd_requested_vram,
            amd_requested_gtt: self.amd_requested_gtt,
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
            vcn_unified,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct ProcUsage {
    pub pid: i32,
    pub name: String,
    pub ids_count: usize,
    pub usage: FdInfoUsage,
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
    pub fn get_cpu_usage(&mut self, pid: i32, name: &str) -> f32 {
        const OFFSET: usize = 3;
        // ref: https://manpages.org/proc/5
        const UTIME: usize = 14 - OFFSET;
        const HZ: f32 = 100.0;
        let Ok(s) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else { return 0.0 };
        // for process names with spaces
        let s = s.trim_start_matches(&format!("{pid} ({name}) "));
        let mut split = s.split(' ').skip(UTIME);
        let [utime, stime] = [split.next(), split.next()]
            .map(|t| t.and_then(|tt| tt.parse::<f32>().ok()).unwrap_or(0.0));

        // ref: https://stackoverflow.com/questions/16726779/how-do-i-get-the-total-cpu-usage-of-an-application-from-proc-pid-stat
        let total_time = (utime + stime) / HZ; // sec = (tick + tick) / HZ

        if let Some(pre_cpu_time) = self.cpu_time_map.get_mut(&pid) {
            let tmp = total_time - *pre_cpu_time;
            *pre_cpu_time = total_time;

            (tmp * 100.0 / self.interval.as_secs_f32()).ceil()
        } else {
            self.cpu_time_map.insert(pid, total_time);

            0.0
        }
    }

    pub fn get_proc_usage(&mut self, proc_info: &ProcInfo) {
        let pid = proc_info.pid;
        let mut stat = FdInfoUsage::default();
        let mut buf = String::with_capacity(2048);
        let mut ids_count = 0usize;

        for fd in &proc_info.fds {
            buf.clear();

            {
                let path = format!("/proc/{pid}/fdinfo/{fd}");
                let Ok(mut f) = fs::File::open(&path) else { continue };
                if f.read_to_string(&mut buf).is_err() { continue }
            }

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
                    "amd-evicte" => stat.evicted_vram_parse(l),
                    "amd-reques" => stat.requested_vram_parse(l),
                    _ => {},
                }
            }
        }

        let mut usage = if let Some(pre_stat) = self.pid_map.get_mut(&pid) {
            // ns -> %
            let usage_per = stat.calc_usage(
                pre_stat,
                &self.interval,
                self.has_vcn,
                self.has_vcn_unified,
            );
            *pre_stat = stat;

            usage_per
        } else {
            let [
                vram_usage,
                gtt_usage,
                system_cpu_memory_usage,
                amd_evicted_vram,
                amd_requested_vram,
                amd_requested_gtt,
            ] = [
                stat.vram_usage,
                stat.gtt_usage,
                stat.system_cpu_memory_usage,
                stat.amd_evicted_vram,
                stat.amd_requested_vram,
                stat.amd_requested_gtt,
            ];

            self.pid_map.insert(pid, stat);

            FdInfoUsage {
                vram_usage,
                gtt_usage,
                system_cpu_memory_usage,
                amd_evicted_vram,
                amd_requested_vram,
                amd_requested_gtt,
                ..Default::default()
            }
        };

        let name = proc_info.name.clone();
        let is_kfd_process = Path::new(KFD_PROC_PATH).join(pid.to_string()).exists();

        usage.cpu = self.get_cpu_usage(pid, &name) as i64;

        self.proc_usage.push(ProcUsage {
            pid,
            name,
            ids_count,
            usage,
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

    pub fn fold_fdinfo_usage(&self) -> (FdInfoUsage, bool, bool, bool) {
        let proc_usage = self.proc_usage
            .iter()
            .fold(FdInfoUsage::default(), |acc, pu| acc + pu.usage);

        (proc_usage, self.has_vcn, self.has_vcn_unified, self.has_vpe)
    }
}
