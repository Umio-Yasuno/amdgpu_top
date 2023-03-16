use crate::{DeviceHandle, SRBM2_OFFSET, Opt};
use crate::util::{BITS, check_register_offset, toggle_view, TopView, TopProgress};

const SRBM2_INDEX: &'static [(&str, usize)] = &[
    ("VCE0", 7),
//    ("VCE1", 14),
    ("SDMA0", 5),
    ("SDMA1", 6),
//    ("SDMA2", 10),
//    ("SDMA3", 11),
];

pub struct SRBM2 {
    pub bits: BITS,
    views: TopProgress,
}

impl SRBM2 {
    pub fn new() -> Self {
        Self {
            bits: BITS::default(),
            views: TopProgress::from(SRBM2_INDEX),
        }
    }

    pub fn dump(&mut self) {
        self.views.set_value(&self.bits);
        self.bits.clear();
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.srbm ^= true;
        }

        siv.call_on_name("SRBM2", toggle_view);
    }

    pub fn top_view(&self) -> TopView {
        self.views.top_view("SRBM2", true)
    }

    pub fn check_reg_offset(amdgpu_dev: &DeviceHandle) -> bool {
        check_register_offset(amdgpu_dev, "mmSRBM_STATUS2", SRBM2_OFFSET)
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(SRBM2_OFFSET) {
            self.bits.acc(out);
        }
    }
}
