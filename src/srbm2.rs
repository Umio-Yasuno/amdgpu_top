use crate::util::Text;
use super::get_bit;

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct SRBM2_BITS {
    sdma0: u8,
    sdma1: u8,
    vce0: u8,
    sdma2: u8,
    sdma3: u8,
    vce1: u8,
}

impl SRBM2_BITS {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn acc(&mut self, reg: u32) {
        self.sdma0 += get_bit!(reg, 5);
        self.sdma1 += get_bit!(reg, 6);
        self.vce0 += get_bit!(reg, 7);
        self.sdma2 += get_bit!(reg, 10);
        self.sdma3 += get_bit!(reg, 11);
        self.vce1 += get_bit!(reg, 14);
    }
}

#[derive(Default)]
pub struct SRBM2 {
    pub flag: bool,
    pub bits: SRBM2_BITS,
    pub text: Text,
}

impl SRBM2 {
    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        if !self.flag {
            return;
        }

        write!(
            self.text.buf,
            concat!(
                " {vce0_name:<30 } => {vce0:3 }%, {vce1_name:<30 } => {vce1:3}% \n",
                " {sdma0_name:<30} => {sdma0:3}%, {sdma1_name:<30} => {sdma1:3}% \n",
                " {sdma2_name:<30} => {sdma2:3}%, {sdma3_name:<30} => {sdma3:3}% \n",
            ),
            vce0_name = "VCE0",
            vce0 = self.bits.vce0,
            vce1_name = "VCE1",
            vce1 = self.bits.vce1,
            sdma0_name = "SDMA0",
            sdma0 = self.bits.sdma0,
            sdma1_name = "SDMA1",
            sdma1 = self.bits.sdma1,
            sdma2_name = "SDMA2",
            sdma2 = self.bits.sdma2,
            sdma3_name = "SDMA3",
            sdma3 = self.bits.sdma3,
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
