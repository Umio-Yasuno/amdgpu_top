use cursive::View;
use cursive::align::HAlign;
use cursive::view::Nameable;
use cursive::views::{
    HideableView,
    LinearLayout,
    NamedView,
    TextContent,
    TextView,
    Panel
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

    pub fn hideable_panel(
        &self,
        title: &str,
        visible: bool,
        index: usize,
    ) -> Panel<NamedView<HideableView<TextView>>> {
        self.hideable_panel_with_name(title, visible, format!("{title} {index}"))
    }

    pub fn hideable_panel_with_name(
        &self,
        title: &str,
        visible: bool,
        name: String,
    ) -> Panel<NamedView<HideableView<TextView>>> {
        let v = TextView::new_with_content(self.content.clone()).no_wrap();

        Panel::new(
            HideableView::new(v)
                .visible(visible)
                .with_name(name)
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

pub fn toggle_view<V: View>(view: &mut HideableView<V>) {
    view.set_visible(!view.is_visible());
}
