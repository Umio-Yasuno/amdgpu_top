use libdrm_amdgpu_sys::AMDGPU::NUM_HBM_INSTANCES;

pub trait IsMax {
    fn is_max(&self) -> bool;
}

impl IsMax for u16 {
    fn is_max(&self) -> bool {
        self == &u16::MAX
    }
}

impl IsMax for u32 {
    fn is_max(&self) -> bool {
        self == &u32::MAX
    }
}

pub fn check_metrics_val<T: IsMax + std::fmt::Display>(val: Option<T>) -> String {
    if let Some(v) = val {
        if v.is_max() { "N/A".to_string() } else { v.to_string() }
    } else {
        "N/A".to_string()
    }
}

pub fn check_temp_array(array: Option<Vec<u16>>) -> Option<Vec<u16>> {
    Some(array?.into_iter().map(|v| if v == u16::MAX { 0 } else { v.saturating_div(100) }).collect())
}

pub fn check_power_clock_array(array: Option<Vec<u16>>) -> Option<Vec<u16>> {
    Some(array?.into_iter().map(|v| if v == u16::MAX { 0 } else { v }).collect())
}

#[allow(non_camel_case_types)]
type HBM_TEMP = Option<[u16; NUM_HBM_INSTANCES as usize]>;

pub fn check_hbm_temp(hbm_temp: HBM_TEMP) -> HBM_TEMP {
    // ref: https://github.com/RadeonOpenCompute/rocm_smi_lib/blob/rocm-5.5.0/include/rocm_smi/rocm_smi.h#L865-L866
    if hbm_temp?.contains(&u16::MAX) {
        None
    } else {
        Some(hbm_temp?.map(|v| v.saturating_div(1_000)))
    }
}
