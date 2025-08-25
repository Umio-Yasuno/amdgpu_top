use libamdgpu_top::{
    AMDGPU::{
        VIDEO_CAPS::CODEC,
        GPU_INFO,
        GpuMetrics,
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
        let node = device_path.card.display();

        json!({
            "Node": node.to_string(),
            "Connectors": Value::Array(vec_conn_info),
        })
    }).collect();

    println!("{}", Value::Array(vec_drm_info_json));
}

pub fn gpu_metrics_json(_title: &str, device_path_list: &[DevicePath]) {
    let vec_metrics_json: Vec<Value> = device_path_list.iter().filter_map(|device_path| {
        let metrics = GpuMetrics::get_from_sysfs_path(&device_path.sysfs_path).ok()?.json();

        Some(json!({
            "device_path": device_path.json(),
            "gpu_metrics": metrics,
        }))
    }).collect();

    println!("{}", Value::Array(vec_metrics_json));
}

pub fn dump_json(device_path_list: &[DevicePath]) {
    let vec_json_info: Vec<Value> = device_path_list.iter().filter_map(|device_path| {
        let amdgpu_dev = device_path.init().ok().unwrap();
        let mut app = AppAmdgpuTop::new(amdgpu_dev, device_path.clone(), &Default::default()).unwrap();

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
    fn json_info(&mut self) -> Value;
    fn stat(&self) -> Value;
}

impl JsonInfo for AppAmdgpuTop {
    fn json_info(&mut self) -> Value {
        let gpu_clk = json!({
            "min": self.device_info.min_gpu_clk,
            "max": self.device_info.max_gpu_clk,
        });
        let mem_clk = json!({
            "min": self.device_info.min_mem_clk,
            "max": self.device_info.max_mem_clk,
        });
        let drm = self.get_drm_version_struct().map_or(Value::Null, |drm| json!({
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
        let power_profiles: Vec<String> = self.device_info.power_profiles
            .iter()
            .map(|p| p.to_string())
            .collect();
        let pp_feature_mask: Vec<String> = libamdgpu_top::PpFeatureMask::get_all_enabled_feature()
            .iter()
            .map(|p| p.to_string())
            .collect();
        let hw_ip_info: Vec<Value> = self.device_info.hw_ip_info_list
            .iter()
            .map(|h| h.json())
            .collect();
        let fw_ver: Vec<Value> = self.device_info.fw_versions
            .iter()
            .filter(|f| f.version != 0)
            .map(|f| f.json())
            .collect();
        let ip_die_entry: Vec<Value> = self.device_info.ip_die_entries
            .iter()
            .map(|f| f.json())
            .collect();

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

        let peak_fp32 = json!({
            "value": self.device_info.ext_info.peak_gflops(),
            "unit": "GFLOPS",
        });
        let peak_pixel = json!({
            "value": self.device_info.ext_info.calc_rop_count() * self.device_info.max_gpu_clk / 1000,
            "unit": "GP/s",
        });
        let peak_mbw = json!({
            "value": self.device_info.ext_info.peak_memory_bw_gb(),
            "unit": "GB/s",
        });

        let json = json!({
            "amdgpu_top_version": amdgpu_top_version(),
            "drm_version": drm,
            "ROCm Version": libamdgpu_top::get_rocm_version(),
            "DeviceName": self.device_info.marketing_name,
            "DevicePath": self.device_path.json(),
            "PCI": self.device_info.pci_bus.to_string(),
            "DeviceID": self.device_info.ext_info.device_id(),
            "RevisionID": self.device_info.ext_info.pci_rev_id(),
            "GPU Type": if self.device_info.ext_info.is_apu() { "APU" } else { "dGPU" },
            "GPU Family": self.device_info.ext_info.get_family_name().to_string(),
            "ASIC Name": self.device_info.ext_info.get_asic_name().to_string(),
            "Chip Class": self.device_info.ext_info.get_chip_class().to_string(),
            "gfx_target_version": match &self.device_info.gfx_target_version {
                Some(ver) => Value::String(ver.to_string()),
                None => Value::Null,
            },
            "Shader Engine": self.device_info.ext_info.max_se(),
            "Shader Array per Shader Engine": self.device_info.ext_info.max_sa_per_se(),
            "CU per Shader Array": json!({
                "min": self.device_info.ext_info.get_min_good_cu_per_sa(),
                "max": self.device_info.ext_info.get_max_good_cu_per_sa(),
            }),
            "Total Compute Unit": self.device_info.ext_info.cu_active_number(),
            "RenderBackend": self.device_info.ext_info.rb_pipes(),
            "RenderBackend Type": if self.device_info.ext_info.get_asic_name().rbplus_allowed() {
                "RB Plus"
            } else {
                "RB"
            },
            "Total ROP": self.device_info.ext_info.calc_rop_count(),
            "GPU Clock": gpu_clk,
            "VRAM Type": self.device_info.ext_info.get_vram_type().to_string(),
            "VRAM Bit width": self.device_info.ext_info.vram_bit_width,
            "VRAM Vendor": match &self.device_info.memory_vendor {
                Some(v) => Value::String(v.clone()),
                None => Value::Null,
            },
            "Memory Clock": mem_clk,
            "ResizableBAR": self.device_info.resizable_bar,
            "VRAM Size": self.device_info.memory_info.vram.total_heap_size,
            "VRAM Usage Size": self.device_info.memory_info.vram.heap_usage,
            "GTT Size": self.device_info.memory_info.gtt.total_heap_size,
            "GTT Usage Size": self.device_info.memory_info.gtt.heap_usage,
            "L1 Cache per CU": self.device_info.l1_cache_size_kib_per_cu << 10,
            "GL1 Cache per Shader Array": self.device_info.gl1_cache_size_kib_per_sa << 10,
            "L2 Cache": self.device_info.total_l2_cache_size_kib << 10,
            "L3 Cache": self.device_info.total_l3_cache_size_mib << 20,
            "Power Cap": power_cap,
            "VBIOS": vbios,
            "Video Caps": video_caps,
            "PCIe Link": link_speed_width,
            "Power Profiles": power_profiles,
            "pp_feature_mask": pp_feature_mask,
            "NPU": self.xdna_device_path.as_ref().map(|x| x.device_name.clone()),
            "Peak FP32": peak_fp32,
            "Peak Pixel Fill-Rate": peak_pixel,
            "Peak Memory Bandwidth": peak_mbw,
            "Hardware IP info": hw_ip_info,
            "Firmware info": fw_ver,
            "IP Discovery table": ip_die_entry,
        });

        json
    }

    fn stat(&self) -> Value {
        json!({
            "VRAM": self.stat.vram_usage.json(),
            "Sensors": self.stat.sensors.as_ref().map(|s| s.json()),
            // "fdinfo": self.stat.fdinfo.json(),
            "gpu_metrics": self.stat.metrics.as_ref().map(|m| m.json()),
            "gpu_activity": self.stat.activity.json(),
        })
    }
}
