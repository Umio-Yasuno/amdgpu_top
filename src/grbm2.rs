use crate::util::{BITS, toggle_view, TopView, TopProgress};
use crate::Opt;

const GRBM2_INDEX: &'static [(&str, usize)] = &[
    ("Texture Cache", 25),
    ("Command Processor -  Fetcher", 28),
    ("Command Processor -  Compute", 29),
    ("Command Processor - Graphics", 30),
];

pub struct GRBM2 {
    pub flag: bool,
    pub bits: BITS,
    views: TopProgress,
}

impl GRBM2 {
    pub fn new() -> Self {
        Self {
            flag: bool::default(),
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
}
