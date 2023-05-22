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

mod sensors;
pub use sensors::*;

mod pcie_bw;
pub use pcie_bw::*;

pub mod gpu_metrics_util;

pub(crate) fn parse_hwmon<T: std::str::FromStr, P: Into<std::path::PathBuf>>(path: P) -> Option<T> {
    std::fs::read_to_string(path.into()).ok()
        .and_then(|file| file.trim_end().parse::<T>().ok())
}
