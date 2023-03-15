/* System Register Block */
use crate::util::{BITS, toggle_view, TopView, TopProgress};
use crate::Opt;

const SRBM_INDEX: &'static [(&str, usize)] = &[
    ("UVD", 19),
];

pub struct SRBM {
    pub flag: bool,
    pub bits: BITS,
    pub views: TopProgress,
}

impl SRBM {
    pub fn new() -> Self {
        Self {
            flag: bool::default(),
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
}
