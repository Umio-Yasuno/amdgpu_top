use std::str::FromStr;
use std::path::PathBuf;
use super::parse_hwmon;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerCapType {
    PPT,
    FastPPT,
    SlowPPT,
}

impl PowerCapType {
    const fn file_names(&self) -> [&str; 4] {
        match self {
            Self::PPT =>
                ["power1_cap", "power1_cap_default", "power1_cap_min", "power1_cap_max"],
            // for VanGogh APU
            Self::FastPPT |
            Self::SlowPPT =>
                ["power2_cap", "power2_cap_default", "power2_cap_min", "power2_cap_max"],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsePowerCapTypeError;

impl FromStr for PowerCapType {
    type Err = ParsePowerCapTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PPT" => Ok(Self::PPT),
            "fastPPT" => Ok(Self::FastPPT),
            "slowPPT" => Ok(Self::SlowPPT),
            _ => Err(ParsePowerCapTypeError),
        }
    }
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

        let label = match std::fs::read_to_string(path.join("power1_label")) {
            Ok(s) => s,
            Err(_) => std::fs::read_to_string(path.join("power2_label")).ok()?,
        };
        let type_ = PowerCapType::from_str(label.as_str().trim_end()).ok()?;
        let [current, default, min, max] = type_.file_names().map(|name| {
            parse_hwmon::<u32, _>(path.join(name)).map(|v| v.saturating_div(1_000_000))
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
