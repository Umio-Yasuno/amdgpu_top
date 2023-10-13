use libamdgpu_top::AMDGPU::{DeviceHandle, drm_amdgpu_memory_info};
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
use super::{PANEL_WIDTH, VRAM_LABEL_WIDTH, TopView};
use libamdgpu_top::VramUsage;

#[derive(Clone, Debug)]
pub struct VramUsageView {
    memory_info: VramUsage,
    vram_counter: Counter,
    gtt_counter: Counter,
    instance: u32,
}

impl VramUsageView {
    const TITLE: &str = "Memory Usage";

    pub fn new(info: &drm_amdgpu_memory_info, instance: u32) -> Self {
        Self {
            memory_info: VramUsage::new(info),
            vram_counter: Counter::new(0),
            gtt_counter: Counter::new(0),
            instance,
        }
    }

    pub fn update_usage(&mut self, amdgpu_dev: &DeviceHandle) {
        self.memory_info.update_usage(amdgpu_dev);
    }

    pub fn view(&self) -> TopView {
        const BAR_WIDTH: usize = PANEL_WIDTH / 2 - VRAM_LABEL_WIDTH;

        let title = Self::TITLE.to_string();
        let label = |value: usize, (_min, max): (usize, usize)| -> String {
            let val = format!("{:5} / {:5} MiB", value >> 20, max >> 20);
            format!("[{val:^width$}]", width = BAR_WIDTH - 2)
        };
        let mut sub_layout = LinearLayout::horizontal();

        for (memory, counter, name) in [
            (&self.memory_info.0.vram, &self.vram_counter, "VRAM"),
            (&self.memory_info.0.gtt, &self.gtt_counter, "GTT"),
        ] {
            sub_layout.add_child(
                FixedLayout::new()
                    .child(
                        Rect::from_size((0, 0), (VRAM_LABEL_WIDTH, 1)),
                        TextView::new(format!(" {name:>4}:")),
                    )
                    .child(
                        Rect::from_size((VRAM_LABEL_WIDTH+1, 0), (BAR_WIDTH, 1)),
                        ProgressBar::new()
                            .with_value(counter.clone())
                            .min(0)
                            .max(memory.total_heap_size as usize)
                            .with_label(label)
                    )
            );
        }

        Panel::new(
            HideableView::new(sub_layout)
                .with_name(vram_view_name(self.instance))
        )
        .title(title)
        .title_position(HAlign::Left)
    }

    pub fn set_value(&self) {
        self.vram_counter.set(self.memory_info.0.vram.heap_usage as usize);
        self.gtt_counter.set(self.memory_info.0.gtt.heap_usage as usize);
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        use crate::{toggle_view, Opt};

        let instances = {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.vram ^= true;

            opt.instances.clone()
        };

        for i in &instances {
            let name = vram_view_name(*i);
            siv.call_on_name(&name, toggle_view);
        }
    }
}

fn vram_view_name(instance: u32) -> String {
    format!("VRAM {instance}")
}
