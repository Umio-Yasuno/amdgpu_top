use std::fmt;

#[allow(non_snake_case)]
mod MaskValue {
    pub const PP_SCLK_DPM_MASK: u32 = 0x1;
    pub const PP_MCLK_DPM_MASK: u32 = 0x2;
    pub const PP_PCIE_DPM_MASK: u32 = 0x4;
    pub const PP_SCLK_DEEP_SLEEP_MASK: u32 = 0x8;
    pub const PP_POWER_CONTAINMENT_MASK: u32 = 0x10;
    pub const PP_UVD_HANDSHAKE_MASK: u32 = 0x20;
    pub const PP_SMC_VOLTAGE_CONTROL_MASK: u32 = 0x40;
    pub const PP_VBI_TIME_SUPPORT_MASK: u32 = 0x80;
    pub const PP_ULV_MASK: u32 = 0x100;
    pub const PP_ENABLE_GFX_CG_THRU_SMU: u32 = 0x200;
    pub const PP_CLOCK_STRETCH_MASK: u32 = 0x400;
    pub const PP_OD_FUZZY_FAN_CONTROL_MASK: u32 = 0x800;
    pub const PP_SOCCLK_DPM_MASK: u32 = 0x1000;
    pub const PP_DCEFCLK_DPM_MASK: u32 = 0x2000;
    pub const PP_OVERDRIVE_MASK: u32 = 0x4000;
    pub const PP_GFXOFF_MASK: u32 = 0x8000;
    pub const PP_ACG_MASK: u32 = 0x10000;
    pub const PP_STUTTER_MODE: u32 = 0x20000;
    pub const PP_AVFS_MASK: u32 = 0x40000;
    pub const PP_GFX_DCS_MASK: u32 = 0x80000;
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
#[repr(u32)]
pub enum PpFeatureMask {
    PP_SCLK_DPM_MASK = MaskValue::PP_SCLK_DPM_MASK,
    PP_MCLK_DPM_MASK = MaskValue::PP_MCLK_DPM_MASK,
    PP_PCIE_DPM_MASK = MaskValue::PP_PCIE_DPM_MASK,
    PP_SCLK_DEEP_SLEEP_MASK = MaskValue::PP_SCLK_DEEP_SLEEP_MASK,
    PP_POWER_CONTAINMENT_MASK = MaskValue::PP_POWER_CONTAINMENT_MASK,
    PP_UVD_HANDSHAKE_MASK = MaskValue::PP_UVD_HANDSHAKE_MASK,
    PP_SMC_VOLTAGE_CONTROL_MASK = MaskValue::PP_SMC_VOLTAGE_CONTROL_MASK,
    PP_VBI_TIME_SUPPORT_MASK = MaskValue::PP_VBI_TIME_SUPPORT_MASK,
    PP_ULV_MASK = MaskValue::PP_ULV_MASK,
    PP_ENABLE_GFX_CG_THRU_SMU = MaskValue::PP_ENABLE_GFX_CG_THRU_SMU,
    PP_CLOCK_STRETCH_MASK = MaskValue::PP_CLOCK_STRETCH_MASK,
    PP_OD_FUZZY_FAN_CONTROL_MASK = MaskValue::PP_OD_FUZZY_FAN_CONTROL_MASK,
    PP_SOCCLK_DPM_MASK = MaskValue::PP_SOCCLK_DPM_MASK,
    PP_DCEFCLK_DPM_MASK = MaskValue::PP_DCEFCLK_DPM_MASK,
    PP_OVERDRIVE_MASK = MaskValue::PP_OVERDRIVE_MASK, // disabled by default
    PP_GFXOFF_MASK = MaskValue::PP_GFXOFF_MASK,
    PP_ACG_MASK = MaskValue::PP_ACG_MASK,
    PP_STUTTER_MODE = MaskValue::PP_STUTTER_MODE,
    PP_AVFS_MASK = MaskValue::PP_AVFS_MASK,
    PP_GFX_DCS_MASK = MaskValue::PP_GFX_DCS_MASK, // disabled by default
}

#[derive(Debug, Clone)]
pub struct UnknownMaskValue;

impl TryFrom<u32> for PpFeatureMask {
    type Error = UnknownMaskValue;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let v = match value {
            MaskValue::PP_SCLK_DPM_MASK => Self::PP_SCLK_DPM_MASK,
            MaskValue::PP_MCLK_DPM_MASK => Self::PP_MCLK_DPM_MASK,
            MaskValue::PP_PCIE_DPM_MASK => Self::PP_PCIE_DPM_MASK,
            MaskValue::PP_SCLK_DEEP_SLEEP_MASK => Self::PP_SCLK_DEEP_SLEEP_MASK,
            MaskValue::PP_POWER_CONTAINMENT_MASK => Self::PP_POWER_CONTAINMENT_MASK,
            MaskValue::PP_UVD_HANDSHAKE_MASK => Self::PP_UVD_HANDSHAKE_MASK,
            MaskValue::PP_SMC_VOLTAGE_CONTROL_MASK => Self::PP_SMC_VOLTAGE_CONTROL_MASK,
            MaskValue::PP_VBI_TIME_SUPPORT_MASK => Self::PP_VBI_TIME_SUPPORT_MASK,
            MaskValue::PP_ULV_MASK => Self::PP_ULV_MASK,
            MaskValue::PP_ENABLE_GFX_CG_THRU_SMU => Self::PP_ENABLE_GFX_CG_THRU_SMU,
            MaskValue::PP_CLOCK_STRETCH_MASK => Self::PP_CLOCK_STRETCH_MASK,
            MaskValue::PP_OD_FUZZY_FAN_CONTROL_MASK => Self::PP_OD_FUZZY_FAN_CONTROL_MASK,
            MaskValue::PP_SOCCLK_DPM_MASK => Self::PP_SOCCLK_DPM_MASK,
            MaskValue::PP_DCEFCLK_DPM_MASK => Self::PP_DCEFCLK_DPM_MASK,
            MaskValue::PP_OVERDRIVE_MASK => Self::PP_OVERDRIVE_MASK,
            MaskValue::PP_GFXOFF_MASK => Self::PP_GFXOFF_MASK,
            MaskValue::PP_ACG_MASK => Self::PP_ACG_MASK,
            MaskValue::PP_STUTTER_MODE => Self::PP_STUTTER_MODE,
            MaskValue::PP_AVFS_MASK => Self::PP_AVFS_MASK,
            MaskValue::PP_GFX_DCS_MASK => Self::PP_GFX_DCS_MASK,
            _ => return Err(UnknownMaskValue),
        };

        Ok(v)
    }
}

impl fmt::Display for PpFeatureMask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PpFeatureMask {
    pub fn get_param_u32() -> Option<u32> {
        let s = std::fs::read_to_string("/sys/module/amdgpu/parameters/ppfeaturemask").ok()?;
        let len = s.len();

        s.get(if s.starts_with("0x") {
            2..len-1
        } else {
            0..len-1
        }).and_then(|param| u32::from_str_radix(param, 16).ok())
    }

    pub fn get_all_enabled_feature() -> Vec<Self> {
        let Some(mut n) = Self::get_param_u32() else { return Vec::new() };
        let mut vec: Vec<Self> = Vec::with_capacity(32);
        let mut i = 0;

        while n != 0 {
            if (n & 0b1) == 1 && let Ok(ftr) = PpFeatureMask::try_from(1 << i) {
                vec.push(ftr);
            }
            n >>= 0b1;
            i += 1;
        }

        vec
    }
}
