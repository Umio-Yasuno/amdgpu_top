use std::time::Duration;
use crate::stat;

// ref: https://github.com/amd/xdna-driver

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd)]
pub struct XdnaFdInfoUsage {
    pub total_memory: u64, // KiB
    pub shared_memory: u64, // KiB
    pub active_memory: u64, // KiB
    // pub resident_memory: u64,
    // pub purgeable_memory: u64,
    pub npu: i64, // ns
}

impl std::ops::Add for XdnaFdInfoUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            total_memory: self.total_memory + other.total_memory,
            shared_memory: self.shared_memory + other.shared_memory,
            active_memory: self.active_memory + other.active_memory,
            npu: self.npu + other.npu,
        }
    }
}

#[test]
fn test_xdna_fdinfo_parse() {
    let mut usage = XdnaFdInfoUsage::default();
    let s = std::fs::read_to_string("src/xdna/fdinfo_sample.txt").unwrap();
    println!("{s}");

    let mut lines = s.lines().skip_while(|l| !l.starts_with("drm-client-id"));

    println!("id: {}", lines.next().and_then(XdnaFdInfoUsage::id_parse).unwrap());

    for l in lines {
        let Some(s) = l.get(0..13) else { continue };

        match s {
            "drm-total-mem" => usage.total_memory_usage_parse(l),
            "drm-shared-me" => usage.shared_memory_usage_parse(l),
            "drm-active-me" => usage.active_memory_usage_parse(l),
            "drm-engine-np" => usage.engine_usage_parse(l),
            _ => {},
        }
    }

    println!("XDNA!: {usage:?}");
}

impl XdnaFdInfoUsage {
    const UNIT_LEN: usize = " KiB".len();

    pub fn id_parse(s: &str) -> Option<usize> {
        const LEN: usize = "drm-client-id:\t".len();
        s.get(LEN..)?.parse().ok()
    }

    fn memory_usage_parse(s: &str, prefix_len: usize) -> Option<u64> {
        let shift = if s.ends_with("MiB") { 10 } else { 0 };

        s.get(prefix_len..s.len()-Self::UNIT_LEN)
            .and_then(|s| s.parse::<u64>().ok())
            .map(|v| v << shift)
    }

    pub fn total_memory_usage_parse(&mut self, s: &str) {
        if let Some(usage) = Self::memory_usage_parse(s, "drm-total-memory:\t".len()) {
            self.total_memory = usage;
        }
    }

    pub fn shared_memory_usage_parse(&mut self, s: &str) {
        if let Some(usage) = Self::memory_usage_parse(s, "drm-shared-memory:\t".len()) {
            self.shared_memory = usage;
        }
    }

    pub fn active_memory_usage_parse(&mut self, s: &str) {
        if let Some(usage) = Self::memory_usage_parse(s, "drm-active-memory:\t".len()) {
            self.active_memory = usage;
        }
    }

    pub fn engine_usage_parse(&mut self, s: &str) {
        const ENGINE_USAGE_PREFIX_LEN: usize = "drm-engine-npu-amdxdna:\t".len();
        const NS: usize = " ns".len();

        let ends = s.len() - NS;
        let Some(ns) = s.get(ENGINE_USAGE_PREFIX_LEN..ends)
            .and_then(|s| s.parse::<i64>().ok()) else { return };

        self.npu += ns;
    }

    pub fn calc_usage(
        &self,
        pre_stat: &Self,
        interval: &Duration,
    ) -> Self {
        let npu = stat::diff_usage(pre_stat.npu, self.npu, interval);

        Self {
            total_memory: self.total_memory,
            shared_memory: self.shared_memory,
            active_memory: self.active_memory,
            npu,
        }
    }
}
