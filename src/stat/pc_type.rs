use super::{DeviceHandle, toggle_view, Opt};
use libdrm_amdgpu_sys::AMDGPU::{
    GRBM_OFFSET,
    GRBM2_OFFSET,
};

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
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
            Self::GRBM => "mmGRBM_STATUS",
            Self::GRBM2 => "mmGRBM2_STATUS2",
        };

        amdgpu_dev.read_mm_registers(offset).map_or_else(|err| {
            println!("{reg_name} ({offset:#X}) register is not allowed. ({err})");
            false
        }, |_| true)
    }

    pub fn cb(&self) -> impl Fn(&mut cursive::Cursive) {
        let name = self.to_string();
        let toggle = match self {
            Self::GRBM => |opt: &mut Opt| {
                let mut opt = opt.lock().unwrap();
                opt.grbm ^= true;
            },
            Self::GRBM2 => |opt: &mut Opt| {
                let mut opt = opt.lock().unwrap();
                opt.grbm2 ^= true;
            },
        };

        move |siv: &mut cursive::Cursive| {
            {
                let opt = siv.user_data::<Opt>().unwrap();
                toggle(opt);
            }

            siv.call_on_name(&name, toggle_view);
        }
    }

}
