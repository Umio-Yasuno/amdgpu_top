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

use libamdgpu_top::stat::{PCType, PerfCounter};
use super::{PANEL_WIDTH, PC_BAR_WIDTH, TopView};

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
        let counters = (0..pc.index.len()).map(|_| Counter::new(0)).collect();

        Self { counters, index }
    }

    pub fn top_view(&self, pc: &PerfCounter, visible: bool) -> TopView {
        const LEFT_LEN: usize = PANEL_WIDTH - PC_BAR_WIDTH;

        let title = pc.pc_type.to_string();
        let mut sub_layout = LinearLayout::vertical();
        let label = |value: usize, (_, _): (usize, usize)| -> String {
            format!("[{val:^width$}]", width = PC_BAR_WIDTH - 2, val = format!("{value:3} %"))
        };

        for (c, (name, _)) in self.counters.iter().zip(pc.index.iter()) {
            sub_layout.add_child(
                FixedLayout::new()
                    .child(
                        Rect::from_size((0,0), (LEFT_LEN+1, 1)),
                        TextView::new(format!("{name:>LEFT_LEN$}:")),
                    )
                    .child(
                        Rect::from_size((LEFT_LEN+2,0), (PC_BAR_WIDTH, 1)),
                        ProgressBar::new()
                            .with_value(c.clone())
                            .with_label(label)
                    )
            );
        }

        Panel::new(
            HideableView::new(sub_layout)
                .visible(visible)
                .with_name(pc_view_name(pc.pc_type, self.index))
        )
        .title(title)
        .title_position(HAlign::Left)
    }

    pub fn set_value(&self, pc: &PerfCounter) {
        for (c, (_, pos)) in self.counters.iter().zip(pc.index.iter()) {
            c.set(pc.bits.get(*pos) as usize)
        }
    }
}

pub fn pc_view_name(pc_type: PCType, index: usize) -> String {
    format!("{pc_type} {index}")
}

pub fn pc_type_cb(pc_type: PCType) -> impl Fn(&mut cursive::Cursive) {
    use crate::{toggle_view, ToggleOptions, Opt};

    let toggle = match pc_type {
        PCType::GRBM => |opt: &mut ToggleOptions| {
            opt.grbm ^= true;
        },
        PCType::GRBM2 => |opt: &mut ToggleOptions| {
            opt.grbm2 ^= true;
        },
    };

    move |siv: &mut cursive::Cursive| {
        let indexes = {
            let opt = siv.user_data::<Opt>().unwrap();
            let mut opt = opt.lock().unwrap();

            toggle(&mut opt);

            opt.indexes.clone()
        };

        for i in &indexes {
            let name = pc_view_name(pc_type, *i);
            siv.call_on_name(&name, toggle_view);
        }
    }
}
