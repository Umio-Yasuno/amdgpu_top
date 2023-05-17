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

#[derive(Clone, Debug)]
pub struct HwmonTemp {
    pub hwmon_path: PathBuf,
    pub type_: HwmonTempType,
    pub current: i64,
    pub critical: Option<i64>,
    pub critical_hyst: Option<i64>,
    pub emergency: Option<i64>,
}

impl HwmonTemp {
    pub fn from_hwmon_path<P: Into<PathBuf>>(path: P, type_: HwmonTempType) -> Option<Self> {
        let path = path.into();

        let names = match type_ {
            HwmonTempType::Edge =>
                ["temp1_input", "temp1_crit", "temp1_crit_hyst", "temp1_emergency"],
            HwmonTempType::Junction =>
                ["temp2_input", "temp2_crit", "temp2_crit_hyst", "temp2_emergency"],
            HwmonTempType::Memory =>
                ["temp3_input", "temp3_crit", "temp3_crit_hyst", "temp3_emergency"],
        };

        let [current, critical, critical_hyst, emergency] = names.map(|name| {
            parse_hwmon(path.join(name)).map(|v: i64| v.saturating_div(1_000))
        });

        Some(Self {
            hwmon_path: path.clone(),
            type_,
            current: current?,
            critical,
            critical_hyst,
            emergency,
        })
    }

    pub fn update<P: Into<PathBuf>>(&mut self, path: P) {
        let name = match self.type_ {
            HwmonTempType::Edge => "temp1_input",
            HwmonTempType::Junction => "temp2_input",
            HwmonTempType::Memory => "temp3_input",
        };
        if let Some(v) = parse_hwmon::<i64, _>(path.into().join(name)) {
            self.current = v.saturating_div(1_000);
        }
    }
}
