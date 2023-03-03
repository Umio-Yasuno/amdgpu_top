pub struct SRBM2 {
    pub sdma0: u8,
    pub sdma1: u8,
    pub vce0: u8,
    pub sdma2: u8,
    pub sdma3: u8,
    pub vce1: u8,
}

impl SRBM2 {
    pub const fn new() -> Self {
        Self {
            sdma0: 0,
            sdma1: 0,
            vce0: 0,
            sdma2: 0,
            sdma3: 0,
            vce1: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new()
    }

    pub fn acc(&mut self, reg: u32) {
        self.sdma0 += ((reg >> 5) & 0b1) as u8;
        self.sdma1 += ((reg >> 6) & 0b1) as u8;
        self.vce0 += ((reg >> 7) & 0b1) as u8;
        self.sdma2 += ((reg >> 10) & 0b1) as u8;
        self.sdma3 += ((reg >> 11) & 0b1) as u8;
        self.vce1 += ((reg >> 14) & 0b1) as u8;
    }

    pub fn stat(&self) -> String {
        format!(
            concat!(
                "\n",
                "{vce0_name:<10} {vce0:3}%\n",
                "{vce1_name:<10} {vce1:3}%\n",
                "{sdma0_name:<10} {sdma0:3}%\n",
                "{sdma1_name:<10} {sdma1:3}%\n",
                "{sdma2_name:<10} {sdma2:3}%\n",
                "{sdma3_name:<10} {sdma3:3}%\n",
            ),
            vce0_name = "VCE0",
            vce0 = self.vce0,
            vce1_name = "VCE1",
            vce1 = self.vce1,
            sdma0_name = "SDMA0",
            sdma0 = self.sdma0,
            sdma1_name = "SDMA1",
            sdma1 = self.sdma1,
            sdma2_name = "SDMA2",
            sdma2 = self.sdma2,
            sdma3_name = "SDMA3",
            sdma3 = self.sdma3,
        )
    }
}
