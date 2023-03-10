/* System Register Block */

use super::get_bit;

#[derive(Default)]
pub struct SRBM {
    pub flag: bool,
    uvd: u8, // Unified Video Decoder
    pub buf: String,
}

impl SRBM {
    pub fn reg_clear(&mut self) {
        self.uvd = 0;
    }

    pub fn acc(&mut self, reg: u32) {
        self.uvd += get_bit!(reg, 19);
    }

    pub fn print(&mut self) {
        use std::fmt::Write;

        self.buf.clear();

        if !self.flag {
            return;
        }

        write!(
            self.buf,
            concat!(
                " {name:<30} => {uvd:3}%",
            ),
            name = "UVD",
            uvd = self.uvd,
        )
        .unwrap();
    }

}
