/* System Register Block */

use crate::util::{BITS, Text};

#[derive(Default)]
pub struct SRBM {
    pub flag: bool,
    // uvd: u8, // Unified Video Decoder
    pub bits: BITS,
    pub text: Text,
}

impl SRBM {
    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        if !self.flag {
            return;
        }

        write!(
            self.text.buf,
            concat!(
                " {name:<30} => {uvd:3}%",
            ),
            name = "UVD",
            uvd = self.bits.0[19],
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
