/// ref: https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gfxoff

/// GFXOFF state is canceled by reading the register, so useful only in SMI mode.

use std::io::{self, Read};
use std::path::PathBuf;
use std::fs;

const BASE: &str = "/sys/kernel/debug/dri";

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GfxoffMode {
    Disable,
    Enable,
    Unknown(u32),
}

impl GfxoffMode {
    pub fn get(instance: u32) -> io::Result<Self> {
        let state = read_gfxoff(format!("{BASE}/{instance}/amdgpu_gfxoff"))?;

        Ok(Self::from(state))
    }

    pub fn is_disabled(&self) -> bool {
        *self == Self::Disable
    }

    pub fn is_enabled(&self) -> bool {
        *self == Self::Enable
    }
}

impl From<u32> for GfxoffMode {
    fn from(val: u32) -> Self {
        match val {
            0 => Self::Disable,
            1 => Self::Enable,
            _ => Self::Unknown(val),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GfxoffStatus {
    InGFXOFF = 0,
    OutGFXOFF = 1,
    NotInGFXOFF = 2,
    IntoGFXOFF = 3,
    Unknown(u32),
}

impl GfxoffStatus {
    pub fn get(instance: u32) -> io::Result<Self> {
        let state = read_gfxoff(format!("{BASE}/{instance}/amdgpu_gfxoff_status"))?;

        Ok(Self::from(state))
    }
}

impl From<u32> for GfxoffStatus {
    fn from(val: u32) -> Self {
        match val {
            0 => Self::InGFXOFF,
            1 => Self::OutGFXOFF,
            2 => Self::NotInGFXOFF,
            3 => Self::IntoGFXOFF,
            _ => Self::Unknown(val),
        }
    }
}

fn read_gfxoff<P: Into<PathBuf>>(path: P) -> io::Result<u32> {
    let mut buf = [0xFFu8; 4];
    
    let mut f = fs::File::open(path.into())?;
    f.read_exact(&mut buf)?;

    Ok(u32::from_le_bytes(buf))
}
