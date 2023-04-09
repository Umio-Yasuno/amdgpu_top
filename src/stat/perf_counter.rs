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

use super::{DeviceHandle, PANEL_WIDTH, PCType, BITS, TopView};

#[derive(Debug)]
pub struct PerfCounter {
    pub pc_type: PCType,
    pub bits: BITS,
    pub counters: Vec<Counter>,
    pub index: Vec<(String, usize)>,
}

impl PerfCounter {
    pub fn new(pc_type: PCType, s: &[(&str, usize)]) -> Self {
        let len = s.len();
        let mut counters: Vec<Counter> = Vec::with_capacity(len);
        let mut index: Vec<(String, usize)>  = Vec::with_capacity(len);

        for (name, idx) in s.iter() {
            counters.push(Counter::new(0));
            index.push((name.to_string(), *idx));
        }

        Self {
            pc_type,
            bits: BITS::default(),
            counters,
            index,
        }
    }

    pub fn top_view(
        &self,
        visible: bool,
    ) -> TopView {
        const BAR_WIDTH: usize = 40;
        const LEFT_LEN: usize = PANEL_WIDTH - BAR_WIDTH;
        
        let title = self.pc_type.to_string();
        let mut sub_layout = LinearLayout::vertical();
        let label = |value: usize, (_, _): (usize, usize)| -> String {
            format!("[{val:^width$}]", width = BAR_WIDTH - 2, val = format!("{value:3} %"))
        };

        for (c, (name, _)) in self.counters.iter().zip(self.index.iter()) {
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
        for (c, (_, pos)) in self.counters.iter().zip(self.index.iter()) {
            c.set(self.bits.get(*pos) as usize)
        }
    }

    pub fn dump(&mut self) {
        self.set_value();
        self.bits.clear();
    }

    pub fn read_reg(&mut self, amdgpu_dev: &DeviceHandle) {
        if let Ok(out) = amdgpu_dev.read_mm_registers(self.pc_type.offset()) {
            self.bits.acc(out);
        }
    }

    pub fn json_value(&mut self) -> Value {
        let mut m = Map::new();

        for (name, pos) in &self.index {
            m.insert(
                name.to_string(),
                json!({
                    "usage": self.bits.get(*pos),
                    "unit": "%",
                }),
            );
        }

        m.into()
    }
}
