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
    amdxdna_drm_get_param_DRM_AMDXDNA_QUERY_FIRMWARE_VERSION as DRM_AMDXDNA_QUERY_FIRMWARE_VERSION,
    amdxdna_drm_get_param_DRM_AMDXDNA_QUERY_AIE_VERSION as DRM_AMDXDNA_QUERY_AIE_VERSION,
    amdxdna_drm_get_param_DRM_AMDXDNA_QUERY_CLOCK_METADATA as DRM_AMDXDNA_QUERY_CLOCK_METADATA,
    amdxdna_drm_ioctl_id_DRM_AMDXDNA_GET_INFO as DRM_AMDXDNA_GET_INFO,
    amdxdna_drm_get_info,
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

    r.and_then(|_| Ok(info))
}

pub unsafe fn get_xdna_clock_metadata(fd: i32) -> Result<XdnaClockMetadata, Errno> {
    let clock_metadata: amdxdna_drm_query_clock_metadata =
        get_xdna_info(fd, DRM_AMDXDNA_QUERY_CLOCK_METADATA)?;

    let clock_metadata = XdnaClockMetadata {
        mp_npu_clock: XdnaClock::from(clock_metadata.mp_npu_clock),
        h_clock: XdnaClock::from(clock_metadata.h_clock),
    };

    Ok(clock_metadata)
}

pub unsafe fn get_xdna_hardware_version(fd: i32) -> Result<amdxdna_drm_query_aie_version, Errno> {
    get_xdna_info(fd, DRM_AMDXDNA_QUERY_AIE_VERSION)
}

pub unsafe fn get_xdna_firmware_version(fd: i32) -> Result<amdxdna_drm_query_firmware_version, Errno> {
    get_xdna_info(fd, DRM_AMDXDNA_QUERY_FIRMWARE_VERSION)
}
