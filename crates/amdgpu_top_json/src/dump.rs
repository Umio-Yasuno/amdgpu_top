use libamdgpu_top::{
    AMDGPU::{
        VIDEO_CAPS::CODEC,
        DeviceHandle,
        GPU_INFO,
    },
    AppDeviceInfo,
    DevicePath,
    stat::Sensors,
};
use serde_json::{json, Map, Value};

pub fn dump_json(device_path_list: &[DevicePath]) {
    let vec_json_info: Vec<Value> = device_path_list.iter().map(|device_path| {
        let Ok(amdgpu_dev) = device_path.init() else { return Value::Null };
        json_info(&amdgpu_dev)
    }).collect();

    println!("{}", Value::Array(vec_json_info));
}

pub fn json_info(amdgpu_dev: &DeviceHandle) -> Value {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let sensors = Sensors::new(amdgpu_dev, &pci_bus, &ext_info);

    let info = AppDeviceInfo::new(amdgpu_dev, &ext_info, &memory_info, &sensors);

    let drm = if let Ok(drm) = amdgpu_dev.get_drm_version_struct() {
        json!({
            "major": drm.version_major,
            "minor": drm.version_minor,
            "patchlevel": drm.version_patchlevel,
        })
    } else {
        Value::Null
    };

    let gpu_clk = json!({
        "min": info.min_gpu_clk,
        "max": info.max_gpu_clk,
    });
    let mem_clk = json!({
        "min": info.min_mem_clk,
        "max": info.max_mem_clk,
    });
    let power_cap = if let Some(cap) = info.power_cap {
        json!({
            "current": cap.current,
            "min": cap.min,
            "max": cap.max,
        })
    } else {
        Value::Null
    };
    let vbios = if let Some(vbios) = info.vbios {
        json!({
            "name": vbios.name,
            "pn": vbios.pn,
            "ver_str": vbios.ver,
            "date": vbios.date,
        })
    } else {
        Value::Null
    };
    let video_caps = if let [Some(decode), Some(encode)] = [info.decode, info.encode] {
        let mut m = Map::new();

        for (codec, dec_cap, enc_cap) in [
            (CODEC::MPEG2, decode.mpeg2, encode.mpeg2),
            (CODEC::MPEG4, decode.mpeg4, encode.mpeg4),
            (CODEC::VC1, decode.vc1, encode.vc1),
            (CODEC::MPEG4_AVC, decode.mpeg4_avc, encode.mpeg4_avc),
            (CODEC::HEVC, decode.hevc, encode.hevc),
            (CODEC::JPEG, decode.jpeg, encode.jpeg),
            (CODEC::VP9, decode.vp9, encode.vp9),
            (CODEC::AV1, decode.av1, encode.av1),
        ] {
            let [dec, enc] = [dec_cap, enc_cap].map(|cap| {
                if let Some(cap) = cap {
                    json!({
                        "width": cap.max_width,
                        "height": cap.max_height,
                    })
                } else {
                    Value::Null
                }
            });

            m.insert(
                codec.to_string(),
                json!({
                    "Decode": dec,
                    "Encode": enc,
                }),
            );
        }

        m.into()
    } else {
        Value::Null
    };

    let json = json!({
        "drm_version": drm,
        "DeviceName": info.marketing_name,
        "PCI": info.pci_bus.to_string(),
        "DeviceID": ext_info.device_id(),
        "RevisionID": ext_info.pci_rev_id(),
        "GPU Type": if ext_info.is_apu() { "APU" } else { "dGPU" },
        "GPU Family": ext_info.get_family_name().to_string(),
        "ASIC Name": ext_info.get_asic_name().to_string(),
        "Chip Class": ext_info.get_chip_class().to_string(),
        "Shader Engine": ext_info.max_se(),
        "Shader Array per Shader Engine": ext_info.max_sa_per_se(),
        "Total Compute Unit": ext_info.cu_active_number(),
        "RenderBackend": ext_info.rb_pipes(),
        "Total ROP": ext_info.calc_rop_count(),
        "GPU Clock": gpu_clk,
        "VRAM Type": ext_info.get_vram_type().to_string(),
        "VRAM Bit width": ext_info.vram_bit_width,
        "Memory Clock": mem_clk,
        "ResizableBAR": info.resizable_bar,
        "VRAM Size": memory_info.vram.total_heap_size,
        "GTT Size": memory_info.gtt.total_heap_size,
        "L1 Cache per CU": info.l1_cache_size_kib_per_cu << 10,
        "GL1 Cache per Shader Array": info.gl1_cache_size_kib_per_sa << 10,
        "L2 Cache": info.total_l2_cache_size_kib << 10,
        "L3 Cache": info.total_l3_cache_size_mib << 20,
        "Power Cap": power_cap,
        "VBIOS": vbios,
        "Video Caps": video_caps,
    });

    json
}
