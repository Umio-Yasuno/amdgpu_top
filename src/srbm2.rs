use crate::util::{BITS, Text};

#[derive(Default)]
pub struct SRBM2 {
    pub flag: bool,
    pub bits: BITS,
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
            vce0 = self.bits.0[7],
            vce1_name = "VCE1",
            vce1 = self.bits.0[14],
            sdma0_name = "SDMA0",
            sdma0 = self.bits.0[5],
            sdma1_name = "SDMA1",
            sdma1 = self.bits.0[6],
            sdma2_name = "SDMA2",
            sdma2 = self.bits.0[10],
            sdma3_name = "SDMA3",
            sdma3 = self.bits.0[11],
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
