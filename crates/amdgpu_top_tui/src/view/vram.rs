use cursive::views::{
    FixedLayout,
    LinearLayout,
    Panel,
    ProgressBar,
    ResizedView,
    TextView,
};
use cursive::view::{Nameable, SizeConstraint};
use cursive::utils::Counter;
use cursive::Rect;
use cursive::align::HAlign;
use super::{PANEL_WIDTH, VRAM_LABEL_WIDTH, ResizedPanel};
use libamdgpu_top::VramUsage;

const TITLE: &str = "Memory Usage";

#[derive(Clone, Debug)]
pub struct VramUsageView {
    vram_counter: Counter,
    gtt_counter: Counter,
    index: usize,
}

impl VramUsageView {
    pub fn new(index: usize) -> Self {
        Self {
            vram_counter: Counter::new(0),
            gtt_counter: Counter::new(0),
            index,
        }
    }

    pub fn resized_panel(&self, usage: &VramUsage) -> ResizedPanel {
        const BAR_WIDTH: usize = PANEL_WIDTH / 2 - VRAM_LABEL_WIDTH;

        let title = TITLE.to_string();
        let label = |value: usize, (_min, max): (usize, usize)| -> String {
            let val = format!("{:5} / {:5} MiB", value >> 20, max >> 20);
            format!("[{val:^width$}]", width = BAR_WIDTH - 2)
        };
        let mut sub_layout = LinearLayout::horizontal();

        for (memory, counter, name) in [
            (&usage.0.vram, &self.vram_counter, "VRAM"),
            (&usage.0.gtt, &self.gtt_counter, "GTT"),
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

        let panel = Panel::new(sub_layout)
            .title(title)
            .title_position(HAlign::Left);

        ResizedView::new(
            SizeConstraint::Free,
            SizeConstraint::Free,
            panel,
        ).with_name(Self::vram_view_name(self.index))
    }

    pub fn set_value(&self, usage: &VramUsage) {
        self.vram_counter.set(usage.0.vram.heap_usage as usize);
        self.gtt_counter.set(usage.0.gtt.heap_usage as usize);
    }

    fn vram_view_name(index: usize) -> String {
        format!("VRAM {index}")
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        use crate::{set_min_height, set_visible_height, Opt};
        use cursive::views::LinearLayout;

        let visible;
        let indexes = {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.vram ^= true;

            visible = opt.vram;

            opt.indexes.clone()
        };

        for i in &indexes {
            let name = Self::vram_view_name(*i);
            if visible {
                siv.call_on_name(&name, set_visible_height::<LinearLayout>);
            } else {
                siv.call_on_name(&name, set_min_height::<LinearLayout>);
            }
        }
    }
}
