use crate::{BASE, HEADING};
use eframe::egui::{self, collapsing_header::CollapsingState, FontId, Id, RichText};

pub fn label(text: &str, font: FontId) -> egui::Label {
    egui::Label::new(RichText::new(text).font(font)).sense(egui::Sense::click())
}

pub fn collapsing_plot(
    ui: &mut egui::Ui,
    text: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), Id::new(text), default_open);

    let _ = ui.horizontal(|ui| {
        let icon = {
            let text = if state.is_open() { "\u{25be}" } else { "\u{25b8}" };
            label(text, BASE)
        };
        let header = label(text, BASE);
        if ui.add(icon).clicked() || ui.add(header).clicked() {
            state.toggle(ui);
        }
    });

    state.show_body_unindented(ui, body);
}

pub fn collapsing_with_id(
    ui: &mut egui::Ui,
    text: &str,
    id: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), Id::new(id), default_open);

    let header_res = ui.horizontal(|ui| {
        let icon = {
            let text = if state.is_open() { "\u{25be}" } else { "\u{25b8}" };
            label(text, HEADING)
        };
        let header = label(text, HEADING);
        if ui.add(icon).clicked() || ui.add(header).clicked() {
            state.toggle(ui);
        }
    });

    state.show_body_indented(&header_res.response, ui, body);
}

pub fn collapsing(
    ui: &mut egui::Ui,
    text: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    collapsing_with_id(ui, text, text, default_open, body);
}

pub fn rt_base<T: Into<String>>(s: T) -> RichText {
    RichText::new(s.into()).font(BASE)
}
