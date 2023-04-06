use cursive::views::{
    HideableView,
    LinearLayout,
    NamedView,
    TextContent,
    TextView,
    Panel
};
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

    pub fn get(&self, index: usize) -> u8 {
        unsafe { *self.0.get_unchecked(index) }
    }
}

impl From<u32> for BITS {
    fn from(val: u32) -> Self {
        let mut out = [0u8; 32];

        for (i, o) in out.iter_mut().enumerate() {
            *o = ((val >> i) & 0b1) as u8;
        }

        Self(out)
    }
}

impl std::ops::AddAssign for BITS {
    fn add_assign(&mut self, other: Self) {
        for (dst, src) in self.0.iter_mut().zip(other.0.iter()) {
            *dst += src;
        }
    }
}

pub type TopView = Panel<NamedView<HideableView<LinearLayout>>>;

pub fn toggle_view(view: &mut HideableView<LinearLayout>) {
    view.set_visible(!view.is_visible());
}
