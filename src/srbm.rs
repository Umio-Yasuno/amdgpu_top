/* System Register Block */

use super::get_bit;
use crate::util::Text;

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct SRBM_BITS(u8);

impl SRBM_BITS {
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn acc(&mut self, reg: u32) {
        self.0 += get_bit!(reg, 19);
    }
}

#[derive(Default)]
pub struct SRBM {
    pub flag: bool,
    // uvd: u8, // Unified Video Decoder
    pub bits: SRBM_BITS,
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
            uvd = self.bits.0,
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
