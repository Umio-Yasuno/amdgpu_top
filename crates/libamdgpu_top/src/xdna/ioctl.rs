#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod bindings {
    include!("bindings.rs");
}
use bindings::{
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE,
    amdxdna_drm_get_info,
    amdxdna_drm_get_power_mode,
    amdxdna_drm_ioctl_id_DRM_AMDXDNA_GET_INFO as DRM_AMDXDNA_GET_INFO,
    amdxdna_drm_get_param_DRM_AMDXDNA_QUERY_AIE_VERSION as DRM_AMDXDNA_QUERY_AIE_VERSION,
    amdxdna_drm_get_param_DRM_AMDXDNA_QUERY_CLOCK_METADATA as DRM_AMDXDNA_QUERY_CLOCK_METADATA,
    amdxdna_drm_get_param_DRM_AMDXDNA_QUERY_FIRMWARE_VERSION as DRM_AMDXDNA_QUERY_FIRMWARE_VERSION,
    amdxdna_drm_get_param_DRM_AMDXDNA_GET_POWER_MODE as DRM_AMDXDNA_GET_POWER_MODE,
    amdxdna_power_mode_type_POWER_MODE_DEFAULT,
    amdxdna_power_mode_type_POWER_MODE_LOW,
    amdxdna_power_mode_type_POWER_MODE_MEDIUM,
    amdxdna_power_mode_type_POWER_MODE_HIGH,
    amdxdna_power_mode_type_POWER_MODE_TURBO,
};
pub use bindings::{
    amdxdna_drm_query_clock,
    amdxdna_drm_query_clock_metadata,
    amdxdna_drm_query_aie_version,
    amdxdna_drm_query_firmware_version,
};

use core::mem::MaybeUninit;
use core::ptr;

use nix::{errno::Errno, ioctl_readwrite};

// red: amdxdna_drm_query_clock
#[derive(Debug, Clone)]
pub struct XdnaClock {
    pub name: String,
    pub freq_mhz: u32,
    pub _pad: u32,
}

impl From<amdxdna_drm_query_clock> for XdnaClock {
    fn from(clock: amdxdna_drm_query_clock) -> Self {
        let name = clock.name.to_vec();
        let name = if let Some(index) = name.iter().position(|&x| x == 0) {
            String::from_utf8(name.get(..index).unwrap_or_default().to_vec())
        } else {
            String::from_utf8(name)
        }.unwrap_or_default();

        Self { name, freq_mhz: clock.freq_mhz, _pad: clock.pad }
    }
}

// ref: amdxdna_drm_query_clock_metadata
#[derive(Debug, Clone)]
pub struct XdnaClockMetadata {
    pub mp_npu_clock: XdnaClock,
    pub h_clock: XdnaClock,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XdnaPowerMode {
    POWER_MODE_DEFAULT = amdxdna_power_mode_type_POWER_MODE_DEFAULT,
    POWER_MODE_LOW = amdxdna_power_mode_type_POWER_MODE_LOW,
    POWER_MODE_MEDIUM = amdxdna_power_mode_type_POWER_MODE_MEDIUM,
    POWER_MODE_HIGH = amdxdna_power_mode_type_POWER_MODE_HIGH,
    POWER_MODE_TURBO = amdxdna_power_mode_type_POWER_MODE_TURBO,
    Invalid(u32),
}

impl From<u32> for XdnaPowerMode {
    fn from(v: u32) -> Self {
        match v {
            amdxdna_power_mode_type_POWER_MODE_DEFAULT => Self::POWER_MODE_DEFAULT,
            amdxdna_power_mode_type_POWER_MODE_LOW => Self::POWER_MODE_LOW,
            amdxdna_power_mode_type_POWER_MODE_MEDIUM => Self::POWER_MODE_MEDIUM,
            amdxdna_power_mode_type_POWER_MODE_HIGH => Self::POWER_MODE_HIGH,
            amdxdna_power_mode_type_POWER_MODE_TURBO => Self::POWER_MODE_TURBO,
            _ => Self::Invalid(v),
        }
    }
}

unsafe fn get_xdna_info<T>(fd: i32, param: u32) -> Result<T, Errno> {
    ioctl_readwrite!(
        xdna_get_info,
        DRM_IOCTL_BASE,
        DRM_COMMAND_BASE + DRM_AMDXDNA_GET_INFO,
        amdxdna_drm_get_info
    );

    let mut arg: MaybeUninit<amdxdna_drm_get_info> = MaybeUninit::zeroed();
    let mut info: MaybeUninit<T> = MaybeUninit::zeroed();
    let arg_ptr = arg.as_mut_ptr();

    {
        ptr::addr_of_mut!((*arg_ptr).param).write(param);
        ptr::addr_of_mut!((*arg_ptr).buffer_size).write(size_of::<T>() as u32);
        ptr::addr_of_mut!((*arg_ptr).buffer).write(info.as_mut_ptr() as u64);
    }

    let r = xdna_get_info(fd, arg_ptr);

    let _ = arg.assume_init();
    let info = info.assume_init();

    r.map(|_| info)
}

pub fn get_xdna_clock_metadata(fd: i32) -> Result<XdnaClockMetadata, Errno> {
    let clock_metadata: amdxdna_drm_query_clock_metadata = unsafe {
        get_xdna_info(fd, DRM_AMDXDNA_QUERY_CLOCK_METADATA)?
    };

    let clock_metadata = XdnaClockMetadata {
        mp_npu_clock: XdnaClock::from(clock_metadata.mp_npu_clock),
        h_clock: XdnaClock::from(clock_metadata.h_clock),
    };

    Ok(clock_metadata)
}

pub fn get_xdna_hardware_version(fd: i32) -> Result<amdxdna_drm_query_aie_version, Errno> {
    unsafe { get_xdna_info(fd, DRM_AMDXDNA_QUERY_AIE_VERSION) }
}

pub fn get_xdna_firmware_version(fd: i32) -> Result<amdxdna_drm_query_firmware_version, Errno> {
    unsafe { get_xdna_info(fd, DRM_AMDXDNA_QUERY_FIRMWARE_VERSION) }
}

pub fn get_xdna_power_mode(fd: i32) -> Result<XdnaPowerMode, Errno> {
    unsafe {
        get_xdna_info::<amdxdna_drm_get_power_mode>(fd, DRM_AMDXDNA_GET_POWER_MODE)
            .map(|v| XdnaPowerMode::from(v.power_mode as u32))
    }
}
