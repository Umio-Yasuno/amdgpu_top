use libdrm_amdgpu_sys::AMDGPU::DeviceHandle;
use crate::Opt;
mod utils;
use utils::*;

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
    ("Vertext Grouper / Tessellator", 17),
    ("Input Assembly", 19),
    ("Work Distributor", 21),
];

pub const GRBM2_INDEX: &[(&str, usize)] = &[
    ("Texture Cache", 25),
    ("Command Processor -  Fetcher", 28),
    ("Command Processor -  Compute", 29),
    ("Command Processor - Graphics", 30),
];

/*
pub const SRBM_INDEX: &[(&str, usize)] = &[
    ("UVD", 19),
];

pub const SRBM2_INDEX: &[(&str, usize)] = &[
    ("VCE0", 7),
//    ("VCE1", 14),
    ("SDMA0", 5),
    ("SDMA1", 6),
//    ("SDMA2", 10),
//    ("SDMA3", 11),
];
*/

pub const CP_STAT_INDEX: &[(&str, usize)] = &[
    ("Prefetch Parser", 15),
    ("Micro Engine", 17),
    // ("Surface Sync", 21),
    ("DMA", 22),
    ("Scratch Memory", 24),
];

mod pc_type;
pub use pc_type::*;

mod perf_counter;
pub use perf_counter::*;

mod pci;
pub use pci::*;

mod fdinfo;
pub use fdinfo::*;

mod vram_usage;
pub use vram_usage::*;

mod sensors;
pub use sensors::*;
