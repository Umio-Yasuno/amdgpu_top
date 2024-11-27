use libdrm_amdgpu_sys::AMDGPU::{
    CHIP_CLASS,
    DeviceHandle,
    GRBM_OFFSET,
    GRBM2_OFFSET,
};
use crate::stat;

#[derive(Clone, Debug)]
pub struct PCIndex {
    pub name: String,
    index: usize,
    pub usage: u8, // %
}

#[derive(Clone, Debug)]
pub struct PerfCounter {
    pub pc_type: PCType,
    bits: PCAcc,
    pub pc_index: Vec<PCIndex>,
}

impl PerfCounter {
    pub fn new_with_chip_class(pc_type: PCType, chip_class: CHIP_CLASS) -> Self {
        let index = match pc_type {
            PCType::GRBM => {
                if CHIP_CLASS::GFX10 <= chip_class {
                    stat::GFX10_GRBM_INDEX
                } else {
                    stat::GRBM_INDEX
                }
            },
            PCType::GRBM2 => {
                if CHIP_CLASS::GFX10 <= chip_class {
                    stat::GFX10_GRBM2_INDEX
                } else if CHIP_CLASS::GFX9 <= chip_class {
                    stat::GFX9_GRBM2_INDEX
                } else {
                    stat::GRBM2_INDEX
                }
            },
        };

        let pc_index = index
            .iter()
            .map(|(name, idx)| {
                let index = *idx;

                assert!(index < 32);

                PCIndex {
                    name: name.to_string(),
                    index,
                    usage: 0,
                }
            })
            .collect();

        Self {
            pc_type,
            bits: PCAcc::default(),
            pc_index,
        }
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(self.pc_type.offset()) {
            self.bits.acc(out);
        }
    }

    pub fn clear_pc(&mut self) {
        self.bits.clear();
    }

    pub fn update_pc_usage(&mut self) {
        for PCIndex { name: _name, index, usage } in self.pc_index.iter_mut() {
            *usage = self.bits.get(*index);
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum PCType {
    GRBM,
    GRBM2,
}

use std::fmt;
impl fmt::Display for PCType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PCType {
    pub const fn offset(&self) -> u32 {
        match self {
            Self::GRBM => GRBM_OFFSET,
            Self::GRBM2 => GRBM2_OFFSET,
        }
    }

    pub fn check_reg_offset(&self, amdgpu_dev: &DeviceHandle) -> bool {
        let offset = self.offset();
        let reg_name = match self {
            Self::GRBM => "GRBM_STATUS",
            Self::GRBM2 => "GRBM2_STATUS2",
        };

        amdgpu_dev.read_mm_registers(offset).map_or_else(|err| {
            println!("{reg_name} ({offset:#X}) register is not allowed. ({err})");
            false
        }, |_| true)
    }
}

#[derive(Clone, Default, Debug)]
struct PCAcc([u8; 32]);

impl PCAcc {
    pub fn clear(&mut self) {
        *self = Self([0u8; 32])
    }

    pub fn acc(&mut self, reg: u32) {
        *self += Self::from(reg)
    }

    fn get(&self, index: usize) -> u8 {
        self.0[index]
    }
}

impl From<u32> for PCAcc {
    fn from(val: u32) -> Self {
        let mut out = [0u8; 32];

        for (i, o) in out.iter_mut().enumerate() {
            *o = ((val >> i) & 0b1) as u8;
        }

        Self(out)
    }
}

impl std::ops::AddAssign for PCAcc {
    fn add_assign(&mut self, other: Self) {
        for (dst, src) in self.0.iter_mut().zip(other.0.iter()) {
            *dst += src;
        }
    }
}
