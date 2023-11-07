use libamdgpu_top::{
    stat,
    AMDGPU::{GpuMetrics, MetricsInfo},
    VramUsage,
};
use stat::{FdInfoStat, GpuActivity, Sensors, PerfCounter};
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

        for (label, idx) in &self.index {
            m.insert(
                label.to_string(),
                json!({
                    "value": self.bits.get(*idx),
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

        m.into()
    }
}

impl OutputJson for FdInfoStat {
    fn json(&self) -> Value {
        let mut m = Map::new();

        for pu in &self.proc_usage {
            let mut sub = Map::new();
            sub.insert(
                "VRAM".to_string(),
                json!({
                    "value": pu.usage.vram_usage >> 10,
                    "unit": "MiB",
                }),
            );
            sub.insert(
                "GTT".to_string(),
                json!({
                    "value": pu.usage.gtt_usage >> 10,
                    "unit": "MiB",
                }),
            );

            let dec_usage = pu.usage.dec + pu.usage.vcn_jpeg;
            let enc_usage = pu.usage.enc + pu.usage.uvd_enc;

            for (label, val) in [
                ("GFX", pu.usage.gfx),
                ("Compute", pu.usage.compute),
                ("DMA", pu.usage.dma),
                ("Decode", dec_usage),
                ("Encode", enc_usage),
                ("CPU", pu.cpu_usage),
                ("Media", pu.usage.media),
            ] {
                sub.insert(
                    label.to_string(),
                    json!({
                        "value": val,
                        "unit": "%",
                    }),
                );
            }
            m.insert(
                format!("{}", pu.pid),
                json!({
                    "name": pu.name,
                    "usage": sub,
                }),
            );
        }

        m.into()
    }
}

impl OutputJson for GpuMetrics {
    fn json(&self) -> Value {
        let mut m = Map::new();

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
