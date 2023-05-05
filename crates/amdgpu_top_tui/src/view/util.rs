use cursive::views::{
    HideableView,
    LinearLayout,
    NamedView,
    TextContent,
    TextView,
    Panel
};
use cursive::align::HAlign;

#[derive(Clone)]
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

pub type TopView = Panel<NamedView<HideableView<LinearLayout>>>;

pub fn toggle_view(view: &mut HideableView<LinearLayout>) {
    view.set_visible(!view.is_visible());
}
