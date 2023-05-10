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

use libamdgpu_top::stat::PCType;
use super::{PANEL_WIDTH, TopView};
use libamdgpu_top::stat::PerfCounter;
/*
use super::toggle_view;
use crate::Opt;
*/

#[derive(Clone, Debug)]
pub struct PerfCounterView {
    pub pc: PerfCounter,
    pub counters: Vec<Counter>,
}

impl PerfCounterView {
    pub fn new(pc_type: PCType, s: &[(&str, usize)]) -> Self {
        let pc = PerfCounter::new(pc_type, s);
        let counters = (0..pc.index.len()).map(|_| Counter::new(0)).collect();

        Self { pc, counters }
    }

    pub fn top_view(
        &self,
        visible: bool,
    ) -> TopView {
        const BAR_WIDTH: usize = 40;
        const LEFT_LEN: usize = PANEL_WIDTH - BAR_WIDTH;
        
        let title = self.pc.pc_type.to_string();
        let mut sub_layout = LinearLayout::vertical();
        let label = |value: usize, (_, _): (usize, usize)| -> String {
            format!("[{val:^width$}]", width = BAR_WIDTH - 2, val = format!("{value:3} %"))
        };

        for (c, (name, _)) in self.counters.iter().zip(self.pc.index.iter()) {
            sub_layout.add_child(
                FixedLayout::new()
                    .child(
                        Rect::from_size((0,0), (LEFT_LEN+1, 1)),
                        TextView::new(format!("{name:>LEFT_LEN$}:")),
                    )
                    .child(
                        Rect::from_size((LEFT_LEN+2,0), (BAR_WIDTH, 1)),
                        ProgressBar::new()
                            .with_value(c.clone())
                            .with_label(label)
                    )
            );
        }

        Panel::new(
            HideableView::new(sub_layout)
                .visible(visible)
                .with_name(&title)
        )
        .title(&title)
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

/*
pub fn pc_type_cb(pc_type: &PCType) -> impl Fn(&mut cursive::Cursive) {
    let name = pc_type.to_string();
    let toggle = match pc_type {
        PCType::GRBM => |opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.grbm ^= true;
        },
        PCType::GRBM2 => |opt: &mut Opt| {
            let mut opt = opt.lock().unwrap();
            opt.grbm2 ^= true;
        },
    };

    move |siv: &mut cursive::Cursive| {
        {
            let opt = siv.user_data::<Opt>().unwrap();
            toggle(opt);
        }

        siv.call_on_name(&name, toggle_view);
    }
}
*/
