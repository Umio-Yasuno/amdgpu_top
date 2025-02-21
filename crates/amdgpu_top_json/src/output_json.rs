use libamdgpu_top::{
    DevicePath,
    stat,
    xdna,
    AMDGPU::{GpuMetrics, MetricsInfo},
    VramUsage,
    PCI,
    ConnectorInfo,
    drmModePropType,
    drmModeModeInfo,
};
use stat::{FdInfoStat, FdInfoUsage, GpuActivity, Sensors, PerfCounter, ProcUsage};
use xdna::{XdnaFdInfoUsage, XdnaFdInfoStat};
use serde_json::{json, Map, Value};
use crate::OutputJson;

impl OutputJson for VramUsage {
    fn json(&self) -> Value {
        let mut m = Map::new();

        for (label, usage) in [
            ("Total VRAM", self.0.vram.total_heap_size >> 20),
            ("Total VRAM Usage", self.0.vram.heap_usage >> 20),
            ("Total GTT", self.0.gtt.total_heap_size >> 20),
            ("Total GTT Usage", self.0.gtt.heap_usage >> 20),
        ] {
            m.insert(
                label.to_string(),
                json!({
                    "value": usage,
                    "unit": "MiB",
                }),
            );
        }

        m.into()
    }
}

impl OutputJson for PerfCounter {
    fn json(&self) -> Value {
        let mut m = Map::new();

        for pc_index in &self.pc_index {
            m.insert(
                pc_index.name.clone(),
                json!({
                    "value": pc_index.usage,
                    "unit": "%",
                }),
            );
        }

        m.into()
    }
}

impl OutputJson for Sensors {
    fn json(&self) -> Value {
        let mut m = Map::new();

        for (label, val, unit) in [
            ("GFX_SCLK", self.sclk, "MHz"),
            ("GFX_MCLK", self.mclk, "MHz"),
            ("VDDNB", self.vddnb, "mV"),
            ("VDDGFX", self.vddgfx, "mV"),
            ("Fan", self.fan_rpm, "RPM"),
            ("Fan Max", self.fan_max_rpm, "RPM"),
        ] {
            m.insert(
                label.to_string(),
                val.map_or(Value::Null, |val| json!({
                    "value": val,
                    "unit": unit,
                })),
            );
        }

        for (label, val) in [
            ("GFX Power", &self.any_hwmon_power()),
            ("Average Power", &self.average_power),
            ("Input Power", &self.input_power),
        ] {
            m.insert(
                label.to_string(),
                val.as_ref().map_or(Value::Null, |power| json!({
                    "value": power.value,
                    "unit": "W",
                })),
            );
        }

        for (label, temp, unit) in [
            ("Edge Temperature", &self.edge_temp, "C"),
            ("Junction Temperature", &self.junction_temp, "C"),
            ("Memory Temperature", &self.memory_temp, "C"),
        ] {
            m.insert(
                label.to_string(),
                temp.as_ref().map_or(Value::Null, |temp| json!({
                    "value": temp.current,
                    "unit": unit,
                })),
            );
        }

        m.insert(
            "PCIe Link Speed".to_string(),
            self.current_link.map_or(Value::Null, |link| link.json()),
        );

        m.insert(
            "PCI Power State".to_string(),
            self.pci_power_state.clone().map_or(Value::Null, Value::String),
        );

        m.insert(
            "Power Profile".to_string(),
            self.power_profile.map_or(Value::Null, |pp| Value::String(pp.to_string())),
        );

        m.into()
    }
}

impl OutputJson for FdInfoUsage {
    fn json(&self) -> Value {
        let mut sub = Map::new();
        sub.insert(
            "VRAM".to_string(),
            json!({
                "value": self.vram_usage >> 10,
                "unit": "MiB",
            }),
        );
        sub.insert(
            "GTT".to_string(),
            json!({
                "value": self.gtt_usage >> 10,
                "unit": "MiB",
            }),
        );

        for (label, val) in [
            ("GFX", self.gfx),
            ("Compute", self.compute),
            ("DMA", self.dma),
            ("Decode", self.total_dec),
            ("Encode", self.total_enc),
            ("Media", self.media),
            ("VCN_JPEG", self.vcn_jpeg),
            ("VPE", self.vpe),
        ] {
            sub.insert(
                label.to_string(),
                json!({
                    "value": val,
                    "unit": "%",
                })
            );
        }

        sub.into()
    }
}

pub trait FdInfoJson {
    fn usage_json(&self, has_vcn: bool, has_vcn_unified: bool, has_vpe: bool) -> Value;
}

impl FdInfoJson for ProcUsage {
    fn usage_json(&self, has_vcn: bool, has_vcn_unified: bool, has_vpe: bool) -> Value {
        let mut sub = Map::new();
        sub.insert(
            "VRAM".to_string(),
            json!({
                "value": self.usage.vram_usage >> 10,
                "unit": "MiB",
            }),
        );
        sub.insert(
            "GTT".to_string(),
            json!({
                "value": self.usage.gtt_usage >> 10,
                "unit": "MiB",
            }),
        );

        for (label, val) in [
            ("GFX", Some(self.usage.gfx)),
            ("Compute", Some(self.usage.compute)),
            ("DMA", Some(self.usage.dma)),
            ("Decode", if !has_vcn_unified { Some(self.usage.total_dec) } else { None }),
            ("Encode", if !has_vcn_unified { Some(self.usage.total_enc) } else { None }),
            ("CPU", Some(self.cpu_usage)),
            ("Media", Some(self.usage.media)),
            ("VCN_JPEG", if has_vcn { Some(self.usage.vcn_jpeg) } else { None }),
            ("VPE", if has_vpe { Some(self.usage.vpe) } else { None }),
        ] {
            sub.insert(
                label.to_string(),
                if let Some(val) = val {
                    json!({
                        "value": val,
                        "unit": "%",
                    })
                } else {
                    Value::Null
                },
            );
        }

        json!({
            "name": self.name,
            "usage": sub,
        })
    }
}

impl OutputJson for FdInfoStat {
    fn json(&self) -> Value {
        let mut m = Map::new();
        let has_vcn = self.has_vcn;
        let has_vcn_unified = self.has_vcn_unified;
        let has_vpe = self.has_vpe;

        for pu in &self.proc_usage {
            if pu.ids_count == 0 { continue; }

            m.insert(
                format!("{}", pu.pid),
                json!({
                    "name": pu.name,
                    "usage": pu.usage_json(has_vcn, has_vcn_unified, has_vpe),
                }),
            );
        }

        m.into()
    }
}

impl OutputJson for XdnaFdInfoUsage {
    fn json(&self) -> Value {
        let mut sub = Map::new();
        sub.insert(
            "Total Memory Usage".to_string(),
            json!({
                "value": self.total_memory >> 10,
                "unit": "MiB",
            }),
        );
        sub.insert(
            "Shared Memory Usage".to_string(),
            json!({
                "value": self.shared_memory >> 10,
                "unit": "MiB",
            }),
        );
        sub.insert(
            "Active Memory Usage".to_string(),
            json!({
                "value": self.active_memory >> 10,
                "unit": "MiB",
            }),
        );

        sub.insert(
            "NPU".to_string(),
            json!({
                "value": self.npu,
                "unit": "%",
            })
        );

        sub.into()
    }
}

impl OutputJson for XdnaFdInfoStat {
    fn json(&self) -> Value {
        let mut m = Map::new();

        for pu in &self.proc_usage {
            if pu.ids_count == 0 { continue; }

            m.insert(
                format!("{}", pu.pid),
                json!({
                    "name": pu.name,
                    "usage": pu.usage.json(),
                }),
            );
        }

        m.into()
    }
}

impl OutputJson for GpuMetrics {
    fn json(&self) -> Value {
        let mut m = Map::new();

        if let Some(header) = self.get_header() {
            m.insert(
                "header".to_string(),
                json!({
                    "structure_size": header.structure_size,
                    "format_revision": header.format_revision,
                    "content_revision": header.content_revision,
                }),
            );
        }

        for (name, val) in [
            ("temperature_edge", self.get_temperature_edge()),
            ("temperature_hotspot", self.get_temperature_hotspot()),
            ("temperature_mem", self.get_temperature_mem()),
            ("temperature_gfx", self.get_temperature_gfx()),
            ("temperature_soc", self.get_temperature_soc()),
            ("temperature_vrgfx", self.get_temperature_vrgfx()),
            ("temperature_vrsoc", self.get_temperature_vrsoc()),
            ("temperature_vrmem", self.get_temperature_vrmem()),
            ("average_cpu_power", self.get_average_cpu_power()),
            ("average_soc_power", self.get_average_soc_power()),
            // ("average_core_power", self.get_average_core_power()),
            ("average_gfx_power", self.get_average_gfx_power()),
            ("average_gfxclk_frequency", self.get_average_gfxclk_frequency()),
            ("average_socclk_frequency", self.get_average_socclk_frequency()),
            ("average_uclk_frequency", self.get_average_uclk_frequency()),
            ("average_fclk_frequency", self.get_average_fclk_frequency()),
            ("average_vclk_frequency", self.get_average_vclk_frequency()),
            ("average_dclk_frequency", self.get_average_dclk_frequency()),
            ("average_vclk1_frequency", self.get_average_vclk1_frequency()),
            ("average_dclk1_frequency", self.get_average_dclk1_frequency()),
            ("current_gfxclk", self.get_current_gfxclk()),
            ("current_socclk", self.get_current_socclk()),
            ("current_uclk", self.get_current_uclk()),
            ("current_fclk", self.get_current_fclk()),
            ("current_vclk", self.get_current_vclk()),
            ("current_dclk", self.get_current_dclk()),
            ("current_vclk1", self.get_current_vclk1()),
            ("current_dclk1", self.get_current_dclk1()),
            ("voltage_gfx", self.get_voltage_gfx()),
            ("voltage_soc", self.get_voltage_soc()),
            ("voltage_mem", self.get_voltage_mem()),
            ("fan_pwm", self.get_fan_pwm()),
            ("pcie_link_width", self.get_pcie_link_width()),
            ("pcie_link_speed", self.get_pcie_link_speed()),
            ("average_cpu_voltage", self.get_average_cpu_voltage()),
            ("average_soc_voltage", self.get_average_soc_voltage()),
            ("average_gfx_voltage", self.get_average_gfx_voltage()),
            ("average_cpu_current", self.get_average_cpu_current()),
            ("average_soc_current", self.get_average_soc_current()),
            ("average_gfx_current", self.get_average_gfx_current()),
        ] {
            m.insert(
                name.to_string(),
                if val == Some(u16::MAX) {
                    Value::Null
                } else {
                    Value::from(val)
                }
            );
        }

        #[allow(clippy::single_element_loop)]
        for (name, val_u32) in [
            ("average_socket_power", self.get_average_socket_power()),
        ] {
            m.insert(
                name.to_string(),
                if val_u32 == Some(u32::MAX) {
                    Value::Null
                } else {
                    Value::from(val_u32)
                }
            );
        }

        for (name, array) in [
            ("temperature_core", self.get_temperature_core()),
            ("temperature_l3", self.get_temperature_l3()),
            ("current_coreclk", self.get_current_coreclk()),
            ("current_l3clk", self.get_current_l3clk()),
            ("average_core_power", self.get_average_core_power()),
            ("average_temperature_core", self.get_average_temperature_core()),
            ("average_temperature_l3", self.get_average_temperature_l3()),
        ] {
            m.insert(
                name.to_string(),
                Value::from(array),
            );
        }

        m.insert(
            "system_clock_counter".to_string(),
            Value::from(self.get_system_clock_counter()),
        );

        m.insert(
            "Throttle Status".to_string(),
            json!(self.get_throttle_status_info().map(|thr|
                thr.get_all_throttler().into_iter()
                    .map(|v| v.to_string()).collect::<Vec<String>>()
            )),
        );

        m.into()
    }
}

impl OutputJson for GpuActivity {
    fn json(&self) -> Value {
        let mut m = Map::new();

        for (s, usage) in [
            ("GFX", &self.gfx),
            ("Memory", &self.umc),
            ("MediaEngine", &self.media),
        ] {
            m.insert(
                s.to_string(),
                json!({
                    "value": usage,
                    "unit": "%",
                }),
            );
        }

        m.into()
    }
}

impl OutputJson for PCI::LINK {
    fn json(&self) -> Value {
        json!({
            "gen": self.r#gen,
            "width": self.width,
        })
    }
}

impl OutputJson for DevicePath {
    fn json(&self) -> Value {
        json!({
            "render": self.render,
            "card": self.card,
            "pci": self.pci.to_string(),
            "DeviceID": self.device_id,
            "RevisionID": self.revision_id,
            "DeviceName": self.device_name,
        })
    }
}

impl OutputJson for ConnectorInfo {
    fn json(&self) -> Value {
        let mut props = Map::new();

        for (prop, value) in &self.mode_props {
            props.insert(
                prop.name.clone(),
                json!({
                    "id": prop.prop_id,
                    "flags": prop.flags,
                    "value": value,
                    "type": prop.prop_type.to_string(),
                    "modes": self.mode_info.iter().map(|m| m.json()).collect::<Vec<Value>>(),
                    "values": if let drmModePropType::RANGE = prop.prop_type {
                        prop.values.clone()
                    } else {
                        Vec::new()
                    },
                    "enums": if let drmModePropType::ENUM = prop.prop_type {
                        let enums: Vec<Value> = prop.enums.iter().map(|enum_| {
                            json!({
                                "name": enum_.name(),
                                "value": enum_.value,
                            })
                        }).collect();

                        enums
                    } else {
                        Vec::new()
                    },
                }),
            );
        }

        json!({
            "id": self.connector_id,
            "type": self.connector_type.to_string(),
            "type_id": self.connector_type_id,
            "connection": self.connection.to_string(),
            "Properties": Value::Object(props),
        })
    }
}

impl OutputJson for drmModeModeInfo {
    fn json(&self) -> Value {
        json!({
            "clock": self.clock,
            "hdisplay": self.hdisplay,
            "hsync_start": self.hsync_start,
            "hsync_end": self.hsync_end,
            "htotal": self.htotal,
            "hskew": self.hskew,
            "vdisplay": self.vdisplay,
            "vsync_start": self.vsync_start,
            "vsync_end": self.vsync_end,
            "vtotal": self.vtotal,
            "vscan": self.vscan,
            "vrefresh": self.vrefresh,
            "flags": self.flags,
            "type": self.type_,
            "name": self.name(),
        })
    }
}
