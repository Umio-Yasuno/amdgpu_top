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

use libamdgpu_top::AMDGPU::CHIP_CLASS;
use libamdgpu_top::stat::{PCType, PerfCounter};
use super::{PANEL_WIDTH, PC_BAR_WIDTH, TopView};

#[derive(Clone, Debug)]
pub struct PerfCounterView {
    pub pc: PerfCounter,
    pub counters: Vec<Counter>,
    instance: u32,
}

impl PerfCounterView {
    pub fn new_with_chip_class(pc_type: PCType, chip_class: CHIP_CLASS, instance: u32) -> Self {
        let pc = PerfCounter::new_with_chip_class(pc_type, chip_class);
        let counters = (0..pc.index.len()).map(|_| Counter::new(0)).collect();

        Self { pc, counters, instance }
    }

    pub fn top_view(
        &self,
        visible: bool,
    ) -> TopView {
        const LEFT_LEN: usize = PANEL_WIDTH - PC_BAR_WIDTH;
        
        let title = self.pc.pc_type.to_string();
        let mut sub_layout = LinearLayout::vertical();
        let label = |value: usize, (_, _): (usize, usize)| -> String {
            format!("[{val:^width$}]", width = PC_BAR_WIDTH - 2, val = format!("{value:3} %"))
        };

        for (c, (name, _)) in self.counters.iter().zip(self.pc.index.iter()) {
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
                .with_name(pc_view_name(self.pc.pc_type, self.instance))
        )
        .title(title)
        .title_position(HAlign::Left)
    }

    pub fn set_value(&self) {
        for (c, (_, pos)) in self.counters.iter().zip(self.pc.index.iter()) {
            c.set(self.pc.bits.get(*pos) as usize)
        }
    }

    pub fn dump(&mut self) {
        self.set_value();
        self.pc.bits.clear();
    }
}

pub fn pc_view_name(pc_type: PCType, instance: u32) -> String {
    format!("{pc_type} {instance}")
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
        let instances = {
            let opt = siv.user_data::<Opt>().unwrap();
            let mut opt = opt.lock().unwrap();

            toggle(&mut opt);

            opt.instances.clone()
        };

        for i in &instances {
            let name = pc_view_name(pc_type, *i);
            siv.call_on_name(&name, toggle_view);
        }
    }
}
