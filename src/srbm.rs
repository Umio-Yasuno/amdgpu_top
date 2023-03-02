pub struct SRBM {
    pub uvd: u8,
}

impl SRBM {
    pub const fn new() -> Self {
        Self {
            uvd: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new()
    }

    pub fn acc(&mut self, reg: u32) {
        self.uvd += ((reg >> 19) & 0b1) as u8;
    }

    pub fn stat(&self) -> String {
        format!(
            "UVD: {uvd}%",
            uvd = self.uvd,
        )
    }
}
