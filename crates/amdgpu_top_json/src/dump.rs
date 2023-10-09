use libamdgpu_top::{
    PCI,
    AMDGPU::{
        VIDEO_CAPS::CODEC,
        DeviceHandle,
        GPU_INFO,
    },
    AppDeviceInfo,
    DevicePath,
    stat::Sensors,
};
use libamdgpu_top::AMDGPU::{drm_amdgpu_info_device, drm_amdgpu_memory_info};
use serde_json::{json, Map, Value};
use crate::{amdgpu_top_version, OutputJson};

pub fn dump_json(device_path_list: &[DevicePath]) {
    let vec_json_info: Vec<Value> = device_path_list.iter().map(|device_path| {
        let Ok(amdgpu_dev) = device_path.init() else { return Value::Null };
        let Ok(pci_bus) = amdgpu_dev.get_pci_bus_info() else { return Value::Null };
        let Ok(ext_info) = amdgpu_dev.device_info() else { return Value::Null };
        let Ok(memory_info) = amdgpu_dev.memory_info() else { return Value::Null };

        json_info(&amdgpu_dev, &pci_bus, &ext_info, &memory_info)
    }).collect();

    println!("{}", Value::Array(vec_json_info));
}

pub fn json_info(
    amdgpu_dev: &DeviceHandle,
    pci_bus: &PCI::BUS_INFO,
    ext_info: &drm_amdgpu_info_device,
    memory_info: &drm_amdgpu_memory_info,
) -> Value {
    let sensors = Sensors::new(amdgpu_dev, &pci_bus, &ext_info);

    let info = AppDeviceInfo::new(amdgpu_dev, &ext_info, &memory_info, &sensors);
    let gpu_clk = json!({
        "min": info.min_gpu_clk,
        "max": info.max_gpu_clk,
    });
    let mem_clk = json!({
        "min": info.min_mem_clk,
        "max": info.max_mem_clk,
    });
    let drm = amdgpu_dev.get_drm_version_struct().map_or(Value::Null, |drm| json!({
        "major": drm.version_major,
        "minor": drm.version_minor,
        "patchlevel": drm.version_patchlevel,
    }));
    let power_cap = info.power_cap.map_or(Value::Null, |cap| json!({
        "current": cap.current,
        "min": cap.min,
        "max": cap.max,
    }));
    let vbios = info.vbios.map_or(Value::Null, |vbios| json!({
        "name": vbios.name,
        "pn": vbios.pn,
        "ver_str": vbios.ver,
        "date": vbios.date,
    }));
    let power_profiles: Vec<String> = info.power_profiles.iter().map(|p| p.to_string()).collect();

    let link_speed_width = if sensors.is_apu {
        Value::Null
    } else {
        json!({
            "min_dpm_link": sensors.min_dpm_link.map_or(Value::Null, |link| link.json()),
            "max_dpm_link": sensors.max_dpm_link.map_or(Value::Null, |link| link.json()),
            "max_gpu_link": sensors.max_gpu_link.map_or(Value::Null, |link| link.json()),
            "max_system_link": sensors.max_system_link.map_or(Value::Null, |link| link.json()),
        })
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
                cap.map_or(Value::Null, |cap| json!({
                    "width": cap.max_width,
                    "height": cap.max_height,
                }))
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
        "amdgpu_top_version": amdgpu_top_version(),
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
        "PCIe Link": link_speed_width,
        "Power Profiles": power_profiles,
    });

    json
}

impl OutputJson for PCI::LINK {
    fn json(&self) -> Value {
        json!({
            "gen": self.gen,
            "width": self.width,
        })
    }
}
