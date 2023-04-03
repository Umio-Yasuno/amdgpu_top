use cursive::views::{
    FixedLayout,
    HideableView,
    LinearLayout,
    NamedView,
    Panel,
    ProgressBar,
    TextView,
};
use cursive::view::Nameable;
use cursive::utils::Counter;
use cursive::Rect;
use cursive::align::HAlign;
use std::fmt::{self, Write};

pub type TopView = Panel<NamedView<HideableView<LinearLayout>>>;

use super::{DeviceHandle, PCType, BITS};

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
        const LEFT_LEN: usize = 30;
        const BAR_WIDTH: usize = 30;
        
        let title = self.pc_type.to_string();
        let mut sub_layout = LinearLayout::vertical();
        let label = |value: usize, (_, _): (usize, usize)| -> String {
            format!("{value:3} %")
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

    pub fn json(&mut self) -> Result<String, fmt::Error> {
        let mut out = format!("\t\"{}\": {{\n", self.pc_type);

        for (name, pos) in &self.index {
            writeln!(
                out,
                concat!(
                    "\t\t\"{name}\": {{\n",
                    "\t\t\t\"val\": {val},\n",
                    "\t\t\t\"unit\": \"%\"\n",
                    "\t\t}},",
                ),
                name = name,
                val = self.bits.get(*pos),
            )?;
        }
        out.pop(); // remove '\n'
        out.pop(); // remove ','
        out.push('\n');

        write!(out, "\t}}")?;

        Ok(out)
    }
}
