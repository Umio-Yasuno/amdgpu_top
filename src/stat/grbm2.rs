use super::{DeviceHandle, Opt};
use super::{BITS, check_register_offset, GRBM2_OFFSET, toggle_view, TopView, TopProgress};

const GRBM2_INDEX: &'static [(&str, usize)] = &[
    ("Texture Cache", 25),
    ("Command Processor -  Fetcher", 28),
    ("Command Processor -  Compute", 29),
    ("Command Processor - Graphics", 30),
];

pub struct GRBM2 {
    pub bits: BITS,
    views: TopProgress,
}

impl GRBM2 {
    pub fn new() -> Self {
        Self {
            bits: BITS::default(),
            views: TopProgress::from(GRBM2_INDEX),
        }
    }

    pub fn dump(&mut self) {
        self.views.set_value(&self.bits);
        self.bits.clear();
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.grbm2 ^= true;
        }

        siv.call_on_name("GRBM2", toggle_view);
    }

    pub fn top_view(&self) -> TopView {
        self.views.top_view("GRBM2", true)
    }

    pub fn check_reg_offset(amdgpu_dev: &DeviceHandle) -> bool {
        check_register_offset(amdgpu_dev, "mmGRBM_STATUS2", GRBM2_OFFSET)
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(GRBM2_OFFSET) {
            self.bits.acc(out);
        }
    }
}
