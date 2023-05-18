pub const GFX10_GRBM_INDEX: &[(&str, usize)] = &[
    ("Graphics Pipe", 31),
    ("Texture Pipe", 14),
    // ("Command Processor", 29),
    // ("Global Data Share", 15),
    ("Shader Export", 20),
    ("Shader Processor Interpolator", 22),
    ("Primitive Assembly", 25),
    ("Depth Block", 26),
    ("Color Block", 30),
    ("Geometry Engine", 21),
];

pub const GRBM_INDEX: &[(&str, usize)] = &[
    ("Graphics Pipe", 31),
    ("Texture Pipe", 14),
    // ("Command Processor", 29),
    // ("Global Data Share", 15),
    ("Shader Export", 20),
    ("Shader Processor Interpolator", 22),
    ("Primitive Assembly", 25),
    ("Depth Block", 26),
    ("Color Block", 30),
    ("Vertex Grouper / Tessellator", 17),
    ("Input Assembly", 19),
    ("Work Distributor", 21),
];

pub const GRBM2_INDEX: &[(&str, usize)] = &[
    ("Texture Cache", 25),
    ("Command Processor -  Fetcher", 28),
    ("Command Processor -  Compute", 29),
    ("Command Processor - Graphics", 30),
];

mod perf_counter;
pub use perf_counter::*;

mod fdinfo;
pub use fdinfo::*;

mod hwmon_temp;
pub use hwmon_temp::*;

mod sensors;
pub use sensors::*;

mod pcie_bw;
pub use pcie_bw::*;

mod power_cap;
pub use power_cap::*;

pub fn check_metrics_val(val: Option<u16>) -> String {
    if let Some(v) = val {
        if v == u16::MAX { "N/A".to_string() } else { v.to_string() }
    } else {
        "N/A".to_string()
    }
}

pub fn check_temp_array<const N: usize>(array: Option<[u16; N]>) -> Option<[u16; N]> {
    Some(array?.map(|v| if v == u16::MAX { 0 } else { v.saturating_div(100) }))
}

pub fn check_power_clock_array<const N: usize>(array: Option<[u16; N]>) -> Option<[u16; N]> {
    Some(array?.map(|v| if v == u16::MAX { 0 } else { v }))
}

pub(crate) fn parse_hwmon<T: std::str::FromStr, P: Into<std::path::PathBuf>>(path: P) -> Option<T> {
    std::fs::read_to_string(path.into()).ok()
        .and_then(|file| file.trim_end().parse::<T>().ok())
}
