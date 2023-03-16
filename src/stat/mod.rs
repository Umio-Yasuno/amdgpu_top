use crate::Opt;
mod utils;
use utils::*;

use libdrm_amdgpu_sys::AMDGPU::{
    DeviceHandle,
    GRBM_OFFSET,
    GRBM2_OFFSET,
    SRBM_OFFSET,
    SRBM2_OFFSET,
    CP_STAT_OFFSET
};

mod grbm;
pub use grbm::*;

mod grbm2;
pub use grbm2::*;

mod srbm;
pub use srbm::*;

mod srbm2;
pub use srbm2::*;

mod cp_stat;
pub use cp_stat::*;

mod pci;
pub use pci::*;

mod gem_info;
pub use gem_info::*;

mod vram_usage;
pub use vram_usage::*;

mod sensors;
pub use sensors::*;
