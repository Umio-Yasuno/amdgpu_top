use std::fmt;
use std::path::PathBuf;
use super::parse_hwmon;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HwmonTempType {
    Edge,
    Junction,
    Memory,
}

impl fmt::Display for HwmonTempType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl HwmonTempType {
    const fn file_names(&self) -> [&str; 4] {
        match self {
            Self::Edge => ["temp1_input", "temp1_crit", "temp1_crit_hyst", "temp1_emergency"],
            Self::Junction => ["temp2_input", "temp2_crit", "temp2_crit_hyst", "temp2_emergency"],
            Self::Memory => ["temp3_input", "temp3_crit", "temp3_crit_hyst", "temp3_emergency"],
        }
    }

    const fn current_temp_file_name(&self) -> &str {
        match self {
            Self::Edge => "temp1_input",
            Self::Junction => "temp2_input",
            Self::Memory => "temp3_input",
        }
    }
}

#[derive(Clone, Debug)]
pub struct HwmonTemp {
    pub type_: HwmonTempType,
    pub current: i64,
    pub critical: Option<i64>,
    pub critical_hyst: Option<i64>,
    pub emergency: Option<i64>,
}

impl HwmonTemp {
    pub fn from_hwmon_path<P: Into<PathBuf>>(path: P, type_: HwmonTempType) -> Option<Self> {
        let path = path.into();

        let [current, critical, critical_hyst, emergency] = type_.file_names().map(|name| {
            parse_hwmon::<i64, _>(path.join(name)).map(|v| v.saturating_div(1_000))
        });
        let current = current?;

        Some(Self {
            type_,
            current,
            critical,
            critical_hyst,
            emergency,
        })
    }

    pub fn update<P: Into<PathBuf>>(&mut self, path: P) {
        let name = self.type_.current_temp_file_name();
        if let Some(v) = parse_hwmon::<i64, _>(path.into().join(name)) {
            self.current = v.saturating_div(1_000);
        }
    }
}
