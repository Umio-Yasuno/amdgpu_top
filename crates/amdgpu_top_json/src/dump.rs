use libamdgpu_top::{
    AMDGPU::{
        VIDEO_CAPS::CODEC,
        GPU_INFO,
    },
    app::AppAmdgpuTop,
    DevicePath,
};
use serde_json::{json, Map, Value};
use crate::{amdgpu_top_version, OutputJson};

pub fn drm_info_json(device_path_list: &[DevicePath]) {
    let vec_drm_info_json: Vec<Value> = device_path_list.iter().map(|device_path| {
        let vec_conn_info = libamdgpu_top::connector_info(device_path);
        let vec_conn_info = vec_conn_info.iter().map(|conn| conn.json()).collect();

        json!({
            "Node": device_path.card.as_os_str().to_str().unwrap(),
            "Connectors": Value::Array(vec_conn_info),
        })
    }).collect();

    println!("{}", Value::Array(vec_drm_info_json));
}

pub fn gpu_metrics_json(_title: &str, device_path_list: &[DevicePath]) {
    let vec_metrics_json: Vec<Value> = device_path_list.iter().filter_map(|device_path| {
        let amdgpu_dev = device_path.init().ok()?;
        let metrics = amdgpu_dev.get_gpu_metrics().ok()?.json();

        Some(json!({
            "device_path": device_path.json(),
            "gpu_metrics": metrics,
        }))
    }).collect();

    println!("{}", Value::Array(vec_metrics_json));
}

pub fn dump_json(device_path_list: &[DevicePath]) {
    let vec_json_info: Vec<Value> = device_path_list.iter().filter_map(|device_path| {
        let amdgpu_dev = device_path.init().ok()?;
        let app = AppAmdgpuTop::new(amdgpu_dev, device_path.clone(), &Default::default())?;

        let mut m = Map::new();
        let mut info = app.json_info();
        let mut stat = app.stat();

        m.append(info.as_object_mut()?);
        m.append(stat.as_object_mut()?);

        Some(m.into())
    }).collect();

    println!("{}", Value::Array(vec_json_info));
}

pub trait JsonInfo {
    fn json_info(&self) -> Value;
    fn stat(&self) -> Value;
}

impl JsonInfo for AppAmdgpuTop {
    fn json_info(&self) -> Value {
        let gpu_clk = json!({
            "min": self.device_info.min_gpu_clk,
            "max": self.device_info.max_gpu_clk,
        });
        let mem_clk = json!({
            "min": self.device_info.min_mem_clk,
            "max": self.device_info.max_mem_clk,
        });
        let drm = self.amdgpu_dev.get_drm_version_struct().map_or(Value::Null, |drm| json!({
            "major": drm.version_major,
            "minor": drm.version_minor,
            "patchlevel": drm.version_patchlevel,
        }));
        let power_cap = self.device_info.power_cap.as_ref().map_or(Value::Null, |cap| json!({
            "current": cap.current,
            "min": cap.min,
            "max": cap.max,
        }));
        let vbios = self.device_info.vbios.as_ref().map_or(Value::Null, |vbios| json!({
            "name": vbios.name,
            "pn": vbios.pn,
            "ver_str": vbios.ver,
            "date": vbios.date,
        }));
        let power_profiles: Vec<String> = self.device_info.power_profiles.iter().map(|p| p.to_string()).collect();

        let link_speed_width = if self.device_info.ext_info.is_apu() {
            Value::Null
        } else {
            let [min_dpm_link, max_dpm_link, max_gpu_link, max_system_link] = [
                &self.device_info.min_dpm_link,
                &self.device_info.max_dpm_link,
                &self.device_info.max_gpu_link,
                &self.device_info.max_system_link,
            ].map(|link_info| link_info.map_or(Value::Null, |link| link.json()));

            json!({
                "min_dpm_link": min_dpm_link,
                "max_dpm_link": max_dpm_link,
                "max_gpu_link": max_gpu_link,
                "max_system_link": max_system_link,
            })
        };

        let video_caps = if let [Some(decode), Some(encode)] = [
            self.device_info.decode,
            self.device_info.encode,
        ] {
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
            "DeviceName": self.device_info.marketing_name,
            "PCI": self.device_info.pci_bus.to_string(),
            "DeviceID": self.device_info.ext_info.device_id(),
            "RevisionID": self.device_info.ext_info.pci_rev_id(),
            "GPU Type": if self.device_info.ext_info.is_apu() { "APU" } else { "dGPU" },
            "GPU Family": self.device_info.ext_info.get_family_name().to_string(),
            "ASIC Name": self.device_info.ext_info.get_asic_name().to_string(),
            "Chip Class": self.device_info.ext_info.get_chip_class().to_string(),
            "Shader Engine": self.device_info.ext_info.max_se(),
            "Shader Array per Shader Engine": self.device_info.ext_info.max_sa_per_se(),
            "Total Compute Unit": self.device_info.ext_info.cu_active_number(),
            "RenderBackend": self.device_info.ext_info.rb_pipes(),
            "Total ROP": self.device_info.ext_info.calc_rop_count(),
            "GPU Clock": gpu_clk,
            "VRAM Type": self.device_info.ext_info.get_vram_type().to_string(),
            "VRAM Bit width": self.device_info.ext_info.vram_bit_width,
            "Memory Clock": mem_clk,
            "ResizableBAR": self.device_info.resizable_bar,
            "VRAM Size": self.device_info.memory_info.vram.total_heap_size,
            "GTT Size": self.device_info.memory_info.gtt.total_heap_size,
            "L1 Cache per CU": self.device_info.l1_cache_size_kib_per_cu << 10,
            "GL1 Cache per Shader Array": self.device_info.gl1_cache_size_kib_per_sa << 10,
            "L2 Cache": self.device_info.total_l2_cache_size_kib << 10,
            "L3 Cache": self.device_info.total_l3_cache_size_mib << 20,
            "Power Cap": power_cap,
            "VBIOS": vbios,
            "Video Caps": video_caps,
            "PCIe Link": link_speed_width,
            "Power Profiles": power_profiles,
        });

        json
    }

    fn stat(&self) -> Value {
        json!({
            "VRAM": self.stat.vram_usage.json(),
            "Sensors": self.stat.sensors.json(),
            // "fdinfo": self.stat.fdinfo.json(),
            "gpu_metrics": self.stat.metrics.as_ref().map(|m| m.json()),
            "gpu_activity": self.stat.activity.json(),
        })
    }
}
