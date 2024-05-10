use std::fmt;
use std::path::PathBuf;

const POWER1_AVG: &str = "power1_average";
const POWER1_INPUT: &str = "power1_input";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum PowerType {
    Input,
    Average,
}

impl PowerType {
    const fn as_filename(&self) -> &str {
        match self {
            Self::Input => POWER1_INPUT,
            Self::Average => POWER1_AVG,
        }
    }
}

impl fmt::Display for PowerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct HwmonPower {
    pub type_: PowerType,
    pub value: u32, // W
}

impl HwmonPower {
    pub fn from_hwmon_path_with_type<P: Into<PathBuf>>(path: P, type_: PowerType) -> Option<Self> {
        let path = path.into();

        let s = std::fs::read_to_string(path.join(type_.as_filename())).ok()?;
        let value = s.trim_end().parse::<u32>().ok()?.saturating_div(1_000_000);

        Some(Self { type_, value })
    }
}
