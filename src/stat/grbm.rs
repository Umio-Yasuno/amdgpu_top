/* GRBM: Graphics Register Block, Graphics Register Bus Manager? */
/* ref: https://github.com/freedesktop/mesa-r600_demo/blob/master/r600_lib.c */
use super::{DeviceHandle, Opt};
use super::{BITS, check_register_offset, GRBM_OFFSET, toggle_view, TopView, TopProgress};

pub struct GRBM {
    pub bits: BITS,
    views: TopProgress,
}

const GFX10_GRBM_INDEX: &'static [(&str, usize)] = &[
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

const GRBM_INDEX: &'static [(&str, usize)] = &[
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

impl GRBM {
    pub fn new(is_gfx10_plus: bool) -> Self {
        let views = if is_gfx10_plus {
            TopProgress::from(GFX10_GRBM_INDEX)
        } else {
            TopProgress::from(GRBM_INDEX)
        };

        Self {
            views,
            bits: BITS::default(),
        }
    }

    pub fn dump(&mut self) {
        self.views.set_value(&self.bits);
        self.bits.clear();
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.grbm ^= true;
        }

        siv.call_on_name("GRBM", toggle_view);
    }

    pub fn top_view(&self) -> TopView {
        self.views.top_view("GRBM", true)
    }

    pub fn check_reg_offset(amdgpu_dev: &DeviceHandle) -> bool {
        check_register_offset(amdgpu_dev, "mmGRBM_STATUS", GRBM_OFFSET)
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(GRBM_OFFSET) {
            self.bits.acc(out);
        }
    }
}
