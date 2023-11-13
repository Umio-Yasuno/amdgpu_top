/// ref: https://www.kernel.org/doc/html/latest/gpu/amdgpu/thermal.html#gfxoff

/// AMD APU/GPU exits GFXOFF state by reading the performance counter (GRBM, GRBM2),  
/// so useful only in SMI mode.

use std::io::{self, Read};
use std::path::PathBuf;
use std::fs;
use crate::PCI;

const BASE: &str = "/sys/kernel/debug/dri";

#[derive(Debug, Clone)]
pub struct GfxoffMonitor {
    debug_dri_path: PathBuf,
    pub mode: GfxoffMode,
    pub status: GfxoffStatus,
}

impl GfxoffMonitor {
    pub fn new(pci_bus: PCI::BUS_INFO) -> Option<Self> {
        let debug_dri_path = pci_bus.get_debug_dri_path()?;
        let mode = GfxoffMode::get_with_debug_dri_path(&debug_dri_path).ok()?;
        let status = GfxoffStatus::get_with_debug_dri_path(&debug_dri_path).ok()?;

        Some(Self { debug_dri_path, mode, status })
    }

    pub fn update(&mut self) -> io::Result<()> {
        self.mode = GfxoffMode::get_with_debug_dri_path(&self.debug_dri_path)?;
        self.status = if self.mode.is_disabled() {
            GfxoffStatus::NotInGFXOFF
        } else {
            GfxoffStatus::get_with_debug_dri_path(&self.debug_dri_path)?
        };

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GfxoffMode {
    Disable,
    Enable,
    Unknown(u32),
}

impl GfxoffMode {
    pub fn get(instance: u32) -> io::Result<Self> {
        Self::get_with_debug_dri_path(format!("{BASE}/{instance}/"))
    }

    pub fn get_with_debug_dri_path<P: Into<PathBuf>>(path: P) -> io::Result<Self> {
        let mode = read_gfxoff(path.into().join("amdgpu_gfxoff"))?;

        Ok(Self::from(mode))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GfxoffStatus {
    InGFXOFF = 0, // GPU is in GFXOFF state, the gfx engine is powered down.
    OutGFXOFF = 1, // Transition out of GFXOFF state
    NotInGFXOFF = 2,
    IntoGFXOFF = 3, // Transition into GFXOFF state
    Unknown(u32),
}

impl GfxoffStatus {
    pub fn get(instance: u32) -> io::Result<Self> {
        Self::get_with_debug_dri_path(format!("{BASE}/{instance}/"))
    }

    pub fn get_with_debug_dri_path<P: Into<PathBuf>>(path: P) -> io::Result<Self> {
        let status= read_gfxoff(path.into().join("amdgpu_gfxoff_status"))?;

        Ok(Self::from(status))
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
