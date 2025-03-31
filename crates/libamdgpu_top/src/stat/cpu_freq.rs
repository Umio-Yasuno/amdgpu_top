use std::path::PathBuf;
use super::parse_hwmon;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub struct CpuFreqInfo {
    pub core_id: u32,
    pub thread_id: u32,
    pub min: u32,
    pub cur: u32,
    pub max: u32,
}

const BASE_DIR: &str = "/sys/devices/system/cpu";
const CUR_FREQ: &str = "scaling_cur_freq";
const MIN_FREQ: &str = "scaling_min_freq";
const MAX_FREQ: &str = "scaling_max_freq";

impl CpuFreqInfo {
    pub fn get_all_cpu_core_freq_info() -> Vec<Self> {
        const MAX_THREADS: u32 = 64;
        // using HashSet for sorting
        let mut set: HashSet<u32> = HashSet::with_capacity(MAX_THREADS as usize);
        let mut vec: Vec<Self> = Vec::with_capacity(MAX_THREADS as usize);

        for i in 0u32..MAX_THREADS {
            let Some(info) = Self::get_cpu_core_freq_info(i) else { break };
            if set.insert(info.core_id) {
                vec.push(info);
            }
        }

        vec
    }

    pub fn get_cpu_core_freq_info(thread_id: u32) -> Option<Self> {
        let core_id = parse_hwmon(format!("{BASE_DIR}/cpu{thread_id}/topology/core_id"))?;
        let freq_path = PathBuf::from(format!("{BASE_DIR}/cpu{thread_id}/cpufreq/"));
        let [min, cur, max] = [MIN_FREQ, CUR_FREQ, MAX_FREQ].map(|freq_file| {
            parse_hwmon::<u32, _>(freq_path.join(freq_file))
        });

        Some(Self {
            core_id,
            thread_id,
            min: min? / 1000,
            cur: cur? / 1000,
            max: max? / 1000,
        })
    }

    pub fn update_cur_freq(&mut self) {
        let path = format!("{BASE_DIR}/cpu{}/cpufreq/{CUR_FREQ}", self.thread_id);
        if let Some(cur) = parse_hwmon::<u32, _>(path) {
            self.cur = cur / 1000;
        }
    }
}
