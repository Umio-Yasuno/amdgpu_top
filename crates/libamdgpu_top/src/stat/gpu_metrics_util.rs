use libdrm_amdgpu_sys::AMDGPU::NUM_HBM_INSTANCES;

pub fn check_metrics_val(val: Option<u16>) -> String {
    if let Some(v) = val {
        if v == u16::MAX { "N/A".to_string() } else { v.to_string() }
    } else {
        "N/A".to_string()
    }
}

pub fn check_temp_array<const N: usize>(array: Option<[u16; N]>) -> Option<[u16; N]> {
    Some(array?.map(|v| if v == u16::MAX { 0 } else { v.saturating_div(100) }))
}

pub fn check_power_clock_array<const N: usize>(array: Option<[u16; N]>) -> Option<[u16; N]> {
    Some(array?.map(|v| if v == u16::MAX { 0 } else { v }))
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
