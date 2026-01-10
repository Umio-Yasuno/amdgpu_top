// GRBM: Graphics Register Bus Management
// ref: https://rocmdocs.amd.com/en/develop/understand/gpu_arch/mi200_performance_counters.html

pub(crate) const GRBM_INDEX: &[(&str, usize)] = &[
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
    // ("Barycentric Interpolator", 23),
];

pub(crate) const GFX10_GRBM_INDEX: &[(&str, usize)] = &[
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
    // ("Barycentric Interpolator", 23),
];

pub(crate) const GRBM2_INDEX: &[(&str, usize)] = &[
    ("RunList Controller", 24),
    ("Texture Cache", 25),
    // ("Texture Cache Resident", 26),
    ("Command Processor -  Fetcher", 28),
    ("Command Processor -  Compute", 29),
    ("Command Processor - Graphics", 30),
];

pub(crate) const GFX9_GRBM2_INDEX: &[(&str, usize)] = &[
    ("RunList Controller", 24),
    ("Texture Cache", 25),
    // ("Texture Cache Resident", 26),
    ("Unified Translation Cache Level-2", 15), // UTCL2
    ("Efficiency Arbiter", 16), // EA
    ("Render Backend Memory Interface", 17), // RMI
    ("Command Processor -  Fetcher", 28), // CPF
    ("Command Processor -  Compute", 29), // CPC
    ("Command Processor - Graphics", 30), // CPG
];

pub(crate) const GFX10_GRBM2_INDEX: &[(&str, usize)] = &[
    ("RunList Controller", 24),
    ("Texture Cache per Pipe", 25),
    ("Unified Translation Cache Level-2", 15), // UTCL2
    ("Efficiency Arbiter", 16), // EA
    ("Render Backend Memory Interface", 17), // RMI
    ("SDMA", 21),
    ("Command Processor -  Fetcher", 28), // CPF
    ("Command Processor -  Compute", 29), // CPC
    ("Command Processor - Graphics", 30), // CPG
];

pub(crate) const GFX10_3_GRBM2_INDEX: &[(&str, usize)] = &[
    ("RunList Controller", 26),
    ("Texture Cache per Pipe", 27),
    ("Unified Translation Cache Level-2", 15), // UTCL2
    ("Efficiency Arbiter", 16), // EA
    ("Render Backend Memory Interface", 17), // RMI
    ("SDMA", 21),
    ("Command Processor -  Fetcher", 28), // CPF
    ("Command Processor -  Compute", 29), // CPC
    ("Command Processor - Graphics", 30), // CPG
];

pub(crate) const GFX12_GRBM2_INDEX: &[(&str, usize)] = &[
    ("RunList Controller", 26),
    ("Texture Cache per Pipe", 27),
    ("Unified Translation Cache Level-2", 15), // UTCL2
    ("Efficiency Arbiter", 16), // EA
    ("SDMA", 21),
    ("Command Processor -  Fetcher", 28), // CPF
    ("Command Processor -  Compute", 29), // CPC
    ("Command Processor - Graphics", 30), // CPG
];

mod cpu_freq;
pub use cpu_freq::*;

mod perf_counter;
pub use perf_counter::*;

mod fdinfo;
pub use fdinfo::*;

mod sensors;
pub use sensors::*;

mod hwmon_power;
pub(crate) use hwmon_power::*;

mod pcie_bw;
pub use pcie_bw::*;

mod gfxoff_state;
pub use gfxoff_state::*;

mod gpu_activity;
pub use gpu_activity::*;

pub mod gpu_metrics_util;

pub(crate) fn parse_hwmon<T: std::str::FromStr, P: Into<std::path::PathBuf>>(path: P) -> Option<T> {
    std::fs::read_to_string(path.into()).ok()
        .and_then(|file| file.trim_end().parse::<T>().ok())
}
