use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::time::Duration;

use super::XdnaFdInfoUsage;
use crate::stat::ProcInfo;

#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct XdnaProcUsage {
    pub pid: i32,
    pub name: String,
    pub ids_count: usize,
    pub usage: XdnaFdInfoUsage,
    // pub cpu_usage: i64, // %
}

#[derive(Clone, Default)]
pub struct XdnaFdInfoStat {
    pub pid_map: HashMap<i32, XdnaFdInfoUsage>,
    pub drm_client_ids: HashSet<usize>,
    pub proc_usage: Vec<XdnaProcUsage>,
    pub interval: Duration,
    // pub cpu_time_map: HashMap<i32, f32>, // sec
}

impl XdnaFdInfoStat {
    pub fn get_proc_usage(&mut self, proc_info: &ProcInfo) {
        let pid = proc_info.pid;
        let name = &proc_info.name;
        let mut stat = XdnaFdInfoUsage::default();
        let mut buf = String::new();
        let mut ids_count = 0usize;

        for fd in &proc_info.fds {
            buf.clear();

            {
                let path = format!("/proc/{pid}/fdinfo/{fd}");
                let Ok(mut f) = fs::File::open(&path) else { continue };
                if f.read_to_string(&mut buf).is_err() { continue }
            }

            let mut lines = buf.lines().skip_while(|l| !l.starts_with("drm-client-id"));
            if let Some(id) = lines.next().and_then(XdnaFdInfoUsage::id_parse) {
                ids_count += 1;
                if !self.drm_client_ids.insert(id) { continue }
            } else {
                continue;
            }

            for l in lines {
                let Some(s) = l.get(0..13) else { continue };

                match s {
                    "drm-total-mem" => stat.total_memory_usage_parse(l),
                    "drm-shared-me" => stat.shared_memory_usage_parse(l),
                    "drm-active-me" => stat.active_memory_usage_parse(l),
                    "drm-engine-np" => stat.engine_usage_parse(l),
                    _ => {},
                }
            }
        }

        let diff = if let Some(pre_stat) = self.pid_map.get_mut(&pid) {
            let tmp = stat.calc_usage(pre_stat, &self.interval);
            *pre_stat = stat;

            tmp
        } else {
            let usage = XdnaFdInfoUsage {
                total_memory: stat.total_memory,
                shared_memory: stat.shared_memory,
                active_memory: stat.active_memory,
                ..Default::default()
            };

            self.pid_map.insert(pid, stat);

            usage
        };

        self.proc_usage.push(XdnaProcUsage {
            pid,
            name: name.to_string(),
            ids_count,
            usage: diff,
        });
    }

    pub fn get_all_proc_usage(&mut self, proc_index: &[ProcInfo]) {
        self.proc_usage.clear();
        self.drm_client_ids.clear();
        for pu in proc_index {
            self.get_proc_usage(pu);
        }
    }

    pub fn fold_fdinfo_usage(&self) -> XdnaFdInfoUsage {
        self.proc_usage.iter().fold(XdnaFdInfoUsage::default(), |acc, pu| acc + pu.usage)
    }
}
