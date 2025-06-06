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

use libamdgpu_top::stat::{PCType, PerfCounter};
use super::{PANEL_WIDTH, PC_BAR_WIDTH, ResizedPanel};

#[derive(Clone, Debug)]
pub struct PerfCounterView {
    pub counters: Vec<Counter>,
    index: usize,
}

impl PerfCounterView {
    pub fn reserve(index: usize) -> Self {
        let counters = (0..32).map(|_| Counter::new(0)).collect();

        Self { counters, index }
    }

    pub fn new(pc: &PerfCounter, index: usize) -> Self {
        let counters = (0..pc.pc_index.len()).map(|_| Counter::new(0)).collect();

        Self { counters, index }
    }

    pub fn resized_panel(&self, pc: &PerfCounter) -> ResizedPanel {
        const LEFT_LEN: usize = PANEL_WIDTH - PC_BAR_WIDTH;

        let title = pc.pc_type.to_string();
        let mut sub_layout = LinearLayout::vertical();
        let label = |value: usize, (_, _): (usize, usize)| -> String {
            format!("[{val:^width$}]", width = PC_BAR_WIDTH - 2, val = format!("{value:3} %"))
        };

        for (c, pc_index) in self.counters.iter().zip(pc.pc_index.iter()) {
            sub_layout.add_child(
                FixedLayout::new()
                    .child(
                        Rect::from_size((0,0), (LEFT_LEN+1, 1)),
                        TextView::new(format!("{:>LEFT_LEN$}:", pc_index.name)),
                    )
                    .child(
                        Rect::from_size((LEFT_LEN+2,0), (PC_BAR_WIDTH, 1)),
                        ProgressBar::new()
                            .with_value(c.clone())
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
        ).with_name(pc_view_name(pc.pc_type, self.index))
    }

    pub fn set_value(&self, pc: &PerfCounter) {
        for (c, pc_index) in self.counters.iter().zip(pc.pc_index.iter()) {
            c.set(pc_index.usage as usize)
        }
    }
}

pub fn pc_view_name(pc_type: PCType, index: usize) -> String {
    format!("{pc_type} {index}")
}

pub fn pc_type_cb(pc_type: PCType) -> impl Fn(&mut cursive::Cursive) {
    use crate::{set_min_height, set_visible_height, ToggleOptions, Opt};
    use cursive::views::LinearLayout;

    let toggle = match pc_type {
        PCType::GRBM => |opt: &mut ToggleOptions| -> bool {
            opt.grbm ^= true;
            opt.grbm
        },
        PCType::GRBM2 => |opt: &mut ToggleOptions| {
            opt.grbm2 ^= true;
            opt.grbm2
        },
    };

    move |siv: &mut cursive::Cursive| {
        let visible;
        let indexes = {
            let opt = siv.user_data::<Opt>().unwrap();
            let mut opt = opt.lock().unwrap();

            visible = toggle(&mut opt);

            opt.indexes.clone()
        };

        for i in &indexes {
            let name = pc_view_name(pc_type, *i);
            if visible {
                siv.call_on_name(&name, set_visible_height::<LinearLayout>);
            } else {
                siv.call_on_name(&name, set_min_height::<LinearLayout>);
            }
        }
    }
}
