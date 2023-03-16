/* System Register Block */
use super::{DeviceHandle, Opt};
use super::{BITS, check_register_offset, SRBM_OFFSET, toggle_view, TopView, TopProgress};

const SRBM_INDEX: &'static [(&str, usize)] = &[
    ("UVD", 19),
];

pub struct SRBM {
    pub bits: BITS,
    views: TopProgress,
}

impl SRBM {
    pub fn new() -> Self {
        Self {
            bits: BITS::default(),
            views: TopProgress::from(SRBM_INDEX),
        }
    }

    pub fn dump(&mut self) {
        self.views.set_value(&self.bits);
        self.bits.clear();
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.uvd ^= true;
        }

        siv.call_on_name("UVD", toggle_view);
    }

    pub fn top_view(&self) -> TopView {
        self.views.top_view("UVD", true)
    }

    pub fn check_reg_offset(amdgpu_dev: &DeviceHandle) -> bool {
        check_register_offset(amdgpu_dev, "mmSRBM_STATUS", SRBM_OFFSET)
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(SRBM_OFFSET) {
            self.bits.acc(out);
        }
    }
}
