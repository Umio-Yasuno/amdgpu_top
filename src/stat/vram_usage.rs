use super::{Opt, TopView, toggle_view};
use libdrm_amdgpu_sys::AMDGPU::{DeviceHandle, drm_amdgpu_memory_info};
use cursive::views::{
    FixedLayout,
    HideableView,
    LinearLayout,
    Panel,
    ProgressBar,
    TextView,
};
use cursive::view::Nameable;
use cursive::utils::Counter;
use cursive::Rect;
use cursive::align::HAlign;
use serde_json::{json, Map, Value};

#[derive(Clone, Debug)]
pub struct VramUsage {
    pub total: u64,
    pub _usable: u64,
    pub usage: u64,
    pub counter: Counter,
}

pub struct VramUsageView {
    pub vram: VramUsage,
    pub gtt: VramUsage,
}

impl VramUsageView {
    const TITLE: &str = "Memory Usage";

    pub fn new(info: &drm_amdgpu_memory_info) -> Self {
        let vram = VramUsage {
            total: info.vram.total_heap_size,
            _usable: info.vram.usable_heap_size,
            usage: info.vram.heap_usage,
            counter: Counter::new(
                50
            ),
        };
        let gtt = VramUsage {
            total: info.gtt.total_heap_size,
            _usable: info.gtt.usable_heap_size,
            usage: info.gtt.heap_usage,
            counter: Counter::new(
                55
            ),
        };

        Self {
            vram,
            gtt,
        }
    }

    pub fn update_usage(&mut self, amdgpu_dev: &DeviceHandle) {
        if let [Ok(usage_vram), Ok(usage_gtt)] = [
            amdgpu_dev.vram_usage_info(),
            amdgpu_dev.gtt_usage_info(),
        ] {
            self.vram.usage = usage_vram;
            self.gtt.usage = usage_gtt;
        }
    }

    pub fn view(
        &self,
    ) -> TopView {
        const LEFT_LEN: usize = 6;
        const BAR_WIDTH: usize = 30;

        let title = Self::TITLE.to_string();
        let label = |value: usize, (_min, max): (usize, usize)| -> String {
            format!("{:5} / {:5} MiB", value >> 20, max >> 20)
        };
        let mut sub_layout = LinearLayout::horizontal();

        for (usage, name) in [(&self.vram, "VRAM"), (&self.gtt, "GTT")] {
            sub_layout.add_child(
                FixedLayout::new()
                    .child(
                        Rect::from_size((0, 0), (LEFT_LEN, 1)),
                        TextView::new(format!(" {name}:")),
                    )
                    .child(
                        Rect::from_size((LEFT_LEN+1, 0), (BAR_WIDTH, 1)),
                        ProgressBar::new()
                            .with_value(usage.counter.clone())
                            .min(0)
                            .max(usage.total as usize)
                            .with_label(label)
                    )
            );
        }

        Panel::new(
            HideableView::new(sub_layout)
                .with_name(&title)
        )
        .title(&title)
        .title_position(HAlign::Left)
    }

    pub fn set_value(&self) {
        self.vram.counter.set(self.vram.usage as usize);
        self.gtt.counter.set(self.gtt.usage as usize);
    }

    pub fn json_value(&self) -> Value {
        let mut m = Map::new();

        for (label, usage) in [
            ("Total VRAM", self.vram.total >> 20),
            ("Total VRAM Usage", self.vram.usage >> 20),
            ("Total GTT", self.gtt.total >> 20),
            ("Total GTT Usage", self.gtt.usage >> 20),
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

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.vram ^= true;
        }

        siv.call_on_name(Self::TITLE, toggle_view);
    }
}
