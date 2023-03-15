use crate::util::{BITS, TopView, TopProgress, toggle_view};
use crate::Opt;

const SRBM2_INDEX: &'static [(&str, usize)] = &[
    ("VCE0", 7),
//    ("VCE1", 14),
    ("SDMA0", 5),
    ("SDMA1", 6),
//    ("SDMA2", 10),
//    ("SDMA3", 11),
];

pub struct SRBM2 {
    pub flag: bool,
    pub bits: BITS,
    views: TopProgress,
}

impl SRBM2 {
    pub fn new() -> Self {
        Self {
            flag: false,
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
}
