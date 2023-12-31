use std::fmt;

#[derive(Debug, Clone)]
pub struct GfxTargetVersion {
    pub major: u32,
    pub minor: u32,
    pub stepping: u32,
}

impl From<u32> for GfxTargetVersion {
    /// e.g. 90012, 100302
    fn from(value: u32) -> Self {
        let [major, minor, stepping] = [
            value / 10000,
            (value / 100) % 100,
            value % 100,
        ];

        Self { major, minor, stepping }
    }
}

impl fmt::Display for GfxTargetVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "gfx{}{}{:x}", self.major, self.minor, self.stepping)
    }
}
