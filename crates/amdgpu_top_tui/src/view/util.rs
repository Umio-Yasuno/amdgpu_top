use cursive::View;
use cursive::align::HAlign;
use cursive::view::{Nameable, SizeConstraint};
use cursive::views::{
    LinearLayout,
    NamedView,
    TextContent,
    TextView,
    Panel,
    ResizedView,
};

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

    pub fn resized_panel(
        &self,
        title: &str,
        index: usize,
    ) -> NamedView<ResizedView<Panel<TextView>>> {
        self.resized_panel_with_name(title, format!("{title} {index}"))
    }

    pub fn resized_panel_with_name(
        &self,
        title: &str,
        name: String,
    ) -> NamedView<ResizedView<Panel<TextView>>> {
        let v = TextView::new_with_content(self.content.clone()).no_wrap();
        let panel = Panel::new(v)
            .title(title)
            .title_position(HAlign::Left);

        ResizedView::new(
            SizeConstraint::Free,
            SizeConstraint::Free,
            panel,
        ).with_name(name)
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

pub type ResizedPanel = NamedView<ResizedView<Panel<LinearLayout>>>;

pub fn set_visible_height<V: View>(view: &mut ResizedView<Panel<V>>) {
    view.set_height(SizeConstraint::Free);
}

pub fn set_min_height<V: View>(view: &mut ResizedView<Panel<V>>) {
    view.set_height(SizeConstraint::Fixed(1));
}
