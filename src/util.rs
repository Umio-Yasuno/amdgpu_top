use libdrm_amdgpu_sys::*;
use cursive::views::{TextContent, TextView, Panel};
use cursive::align::HAlign;

pub struct Text {
    pub buf: String,
    pub content: TextContent,
}

impl Text {
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn set(&self) {
        self.content.set_content(&self.buf);
    }

    pub fn panel(&self, title: &str) -> Panel<TextView> {
       Panel::new(
            TextView::new_with_content(self.content.clone())
        )
        .title(title)
        .title_position(HAlign::Left)
    }
}

impl Default for Text {
    fn default() -> Self {
        Self {
            buf: String::new(),
            content: TextContent::new(""),
        }
    }
}

#[derive(Default, Debug)]
pub struct BITS(pub [u8; 32]);

impl BITS {
    pub fn clear(&mut self) {
        *self = Self([0u8; 32])
    }

    pub fn acc(&mut self, reg: u32) {
        *self += Self::from(reg)
    }
}

impl From<u32> for BITS {
    fn from(val: u32) -> Self {
        let mut out = [0u8; 32];

        for i in 0usize..32 {
            out[i] = ((val >> i) & 0b1) as u8;
        }

        Self(out)
    }
}

impl std::ops::AddAssign for BITS {
    fn add_assign(&mut self, other: Self) {
        for i in 0usize..32 {
            self.0[i] += other.0[i];
        }
    }
}

use cursive::views::{FixedLayout, HideableView, LinearLayout, ProgressBar, NamedView};
use cursive::view::Nameable;
use cursive::utils::Counter;
use cursive::Rect;

pub type TopView = Panel<NamedView<HideableView<LinearLayout>>>;

pub struct TopProgress {
    pub counters: Vec<Counter>,
    pub index: Vec<(String, usize)>,
}

impl TopProgress {
    pub fn top_view(
        &self,
        title: &str,
        visible: bool,
    ) -> TopView {
        const LEFT_LEN: usize = 30;

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
                        Rect::from_size((LEFT_LEN+2,0), (30, 1)),
                        ProgressBar::new()
                            .with_value(c.clone())
                            .with_label(label)
                    )
            );
        }

        Panel::new(
            HideableView::new(sub_layout)
                .visible(visible)
                .with_name(title)
        )
        .title(title)
        .title_position(HAlign::Left)
    }

    pub fn set_value(&self, bits: &BITS) {
        for (c, (_, pos)) in self.counters.iter().zip(self.index.iter()) {
            c.set(bits.0[*pos] as usize)
        }
    }
}

impl From<&[(&str, usize)]> for TopProgress {
    fn from(s: &[(&str, usize)]) -> Self {
        let len = s.len();
        let mut counters: Vec<Counter> = Vec::with_capacity(len);
        let mut index: Vec<(String, usize)>  = Vec::with_capacity(len);

        for (name, idx) in s.iter() {
            counters.push(Counter::new(0));
            index.push((name.to_string(), *idx));
        }

        Self {
            counters,
            index,
        }
    }
}

pub fn toggle_view(view: &mut HideableView<LinearLayout>) {
    view.set_visible(!view.is_visible());
}

pub fn get_min_clk(
    amdgpu_dev: &AMDGPU::DeviceHandle,
    pci_bus: &PCI::BUS_INFO
) -> (u64, u64) {
    if let [Some(gpu), Some(mem)] = [
        amdgpu_dev.get_min_gpu_clock_from_sysfs(pci_bus),
        amdgpu_dev.get_min_memory_clock_from_sysfs(pci_bus),
    ] {
        (gpu, mem)
    } else {
        (0, 0)
    }
}

pub fn check_register_offset(
    amdgpu_dev: &AMDGPU::DeviceHandle,
    name: &str,
    offset: u32
) -> bool {
    if let Err(err) = amdgpu_dev.read_mm_registers(offset) {
        println!("{name} ({offset:#X}) register is not allowed. ({err})");
        return false;
    }

    true
}

pub fn vbios_info(amdgpu_dev: &AMDGPU::DeviceHandle) {
    if let Ok(vbios) = unsafe { amdgpu_dev.vbios_info() } {
        let [name, pn, ver_str, date] = [
            vbios.name.to_vec(),
            vbios.vbios_pn.to_vec(),
            vbios.vbios_ver_str.to_vec(),
            vbios.date.to_vec(),
        ]
        .map(|v| {
            let tmp = String::from_utf8(v).unwrap();

            tmp.trim_end_matches(|c: char| c.is_control() || c.is_whitespace()).to_string()
        });

        println!("\nVBIOS info:");
        println!("name:\t[{name}]");
        println!("pn:\t[{pn}]");
        println!("ver_str:[{ver_str}]");
        println!("date:\t[{date}]");
    }
}
