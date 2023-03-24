use super::{DeviceHandle, Opt};
use cursive::views::{HideableView, LinearLayout};
use libdrm_amdgpu_sys::AMDGPU::{
    GRBM_OFFSET,
    GRBM2_OFFSET,
    SRBM_OFFSET,
    SRBM2_OFFSET,
    CP_STAT_OFFSET
};

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum PCType {
    GRBM,
    GRBM2,
    SRBM,
    SRBM2,
    CP_STAT,
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
            Self::SRBM => SRBM_OFFSET,
            Self::SRBM2 => SRBM2_OFFSET,
            Self::CP_STAT => CP_STAT_OFFSET,
        }
    }

    pub fn check_reg_offset(&self, amdgpu_dev: &DeviceHandle) -> bool {
        let offset = self.offset();
        let reg_name = match self {
            Self::GRBM => "mmGRBM_STATUS",
            Self::GRBM2 => "mmGRBM2_STATUS2",
            Self::SRBM => "mmSRBM_STATUS",
            Self::SRBM2 => "mmSRBM2_STATUS2",
            Self::CP_STAT => "mmCP_STAT_STATUS",
        };

        if let Err(err) = amdgpu_dev.read_mm_registers(offset) {
            println!("{reg_name} ({offset:#X}) register is not allowed. ({err})");
            return false;
        }

        true
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
            Self::SRBM => |opt: &mut Opt| {
                let mut opt = opt.lock().unwrap();
                opt.uvd ^= true;
            },
            Self::SRBM2 => |opt: &mut Opt| {
                let mut opt = opt.lock().unwrap();
                opt.srbm ^= true;
            },
            Self::CP_STAT => |opt: &mut Opt| {
                let mut opt = opt.lock().unwrap();
                opt.cp_stat ^= true;
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

fn toggle_view(view: &mut HideableView<LinearLayout>) {
    view.set_visible(!view.is_visible());
}
