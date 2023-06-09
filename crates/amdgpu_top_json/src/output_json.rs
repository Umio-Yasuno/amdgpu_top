use libamdgpu_top::{stat, VramUsage};
use stat::{FdInfoStat, Sensors, PerfCounter};
use serde_json::{json, Map, Value};

pub trait OutputJson {
    fn json(&self) -> Value;
}

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
            // ("GFX Temp.", self.temp, "C"),
            ("GFX Power", self.power, "W"),
            ("Fan", self.fan_rpm, "RPM"),
        ] {
            let Some(val) = val else { continue };

            m.insert(
                label.to_string(),
                json!({
                    "value": val,
                    "unit": unit,
                }),
            );
        }

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
                format!("{} ({})", pu.name, pu.pid),
                json!({
                    "usage": sub,
                }),
            );
        }

        m.into()
    }
}

/*
    TODO: GpuMetrics
*/
