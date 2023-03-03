pub struct SRBM {
    // pub mcc: u8, // ?
    // pub mcd: u8, // ?
    pub uvd: u8, // Unified Video Decoder
    // pub bif: u8, // Bus Interface
}

impl SRBM {
    pub const fn new() -> Self {
        Self {
            // mcc: 0,
            // mcd: 0,
            uvd: 0,
            // bif: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new()
    }

    pub fn acc(&mut self, reg: u32) {
        // self.mcc += ((reg >> 11) & 0b1) as u8;
        // self.mcd += ((reg >> 12) & 0b1) as u8;
        self.uvd += ((reg >> 19) & 0b1) as u8;
        // self.bif += ((reg >> 29) & 0b1) as u8;
    }

    pub fn stat(&self) -> String {
        format!(
            concat!(
                "\n",
                " {name:<10} {uvd:3}% \n",
                // "MCC:           {mcc:3}%\n",
                // "MCD:           {mcd:3}%\n",
                // "Bus Interface: {bif}%\n",
            ),
            name = "UVD",
            uvd = self.uvd,
            // mcc = self.mcc,
            // mcd = self.mcd,
            // bif = self.bif,
        )
    }
}
