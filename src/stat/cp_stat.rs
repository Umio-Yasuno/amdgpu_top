use crate::{DeviceHandle, CP_STAT_OFFSET, Opt};
use crate::util::{BITS, check_register_offset, toggle_view, TopView, TopProgress};

const CP_STAT_INDEX: &'static [(&str, usize)] = &[
    ("Prefetch Parser", 15),
    ("Micro Engine", 17),
    // ("Surface Sync", 21),
    ("DMA", 22),
    ("Scratch Memory", 24),
];

#[allow(non_camel_case_types)]
pub struct CP_STAT {
    pub bits: BITS,
    views: TopProgress,
}

impl CP_STAT {
    pub fn new() -> Self {
        Self {
            bits: BITS::default(),
            views: TopProgress::from(CP_STAT_INDEX),
        }
    }

    pub fn dump(&mut self) {
        self.views.set_value(&self.bits);
        self.bits.clear();
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.cp_stat ^= true;
        }

        siv.call_on_name("CP_STAT", toggle_view);
    }

    pub fn top_view(&self, visible: bool) -> TopView {
        self.views.top_view("CP_STAT", visible)
    }

    pub fn check_reg_offset(amdgpu_dev: &DeviceHandle) -> bool {
        check_register_offset(amdgpu_dev, "mmCP_STAT", CP_STAT_OFFSET)
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(CP_STAT_OFFSET) {
            self.bits.acc(out);
        }
    }
}
