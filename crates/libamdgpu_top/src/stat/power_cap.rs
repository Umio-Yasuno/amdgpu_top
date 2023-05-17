use std::path::PathBuf;
use super::parse_hwmon;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerCapType {
    PPT,
    FastPPT,
    SlowPPT,
}

#[derive(Clone, Debug)]
pub struct PowerCap {
    pub type_: PowerCapType,
    pub current: u32,
    pub default: u32,
    pub min: u32,
    pub max: u32,
}

impl PowerCap {
    pub fn from_hwmon_path<P: Into<PathBuf>>(path: P) -> Option<Self> {
        let path = path.into();

        let type_ = match std::fs::read_to_string(path.join("power1_label")).ok()?.as_str() {
            "fastPPT" => PowerCapType::FastPPT,
            "slowPPT" => PowerCapType::SlowPPT,
            _ => PowerCapType::PPT,
        };

        let names = if type_ == PowerCapType::FastPPT || type_ == PowerCapType::SlowPPT {
            // for VanGogh APU
            ["power2_cap", "power2_cap_default", "power2_cap_min", "power2_cap_max"]
        } else {
            ["power1_cap", "power1_cap_default", "power1_cap_min", "power1_cap_max"]
        };

        let [current, default, min, max] = names.map(|name| {
            parse_hwmon(path.join(name)).map(|v: u32| v.saturating_div(1_000_000))
        });

        Some(Self {
            type_,
            current: current?,
            default: default?,
            min: min?,
            max: max?,
        })
    }
}
