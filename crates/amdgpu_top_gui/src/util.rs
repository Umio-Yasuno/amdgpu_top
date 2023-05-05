use crate::{BASE, HEADING, HISTORY_LENGTH};
use eframe::egui::{self, collapsing_header::CollapsingState, FontId, util::History, Id, RichText};
use libamdgpu_top::{DevicePath, PCI, stat::Sensors};

pub struct DeviceListMenu {
    pub instance: u32,
    pub name: String,
    pub pci: PCI::BUS_INFO,
}

impl DeviceListMenu {
    pub fn new(device_path: &DevicePath) -> Option<Self> {
        let instance = device_path.get_instance_number()?;
        let pci = device_path.pci?;
        let name = {
            let amdgpu_dev = device_path.init().ok()?;
            amdgpu_dev.get_marketing_name().unwrap_or_default()
        };

        Some(Self { instance, pci, name })
    }
}

#[derive(Debug, Clone)]
pub struct SensorsHistory {
    pub sclk: History<u32>,
    pub mclk: History<u32>,
    pub vddgfx: History<u32>,
    pub vddnb: History<u32>,
    pub temp: History<u32>,
    pub power: History<u32>,
    pub fan_rpm: History<u32>,
}

impl SensorsHistory {
    pub fn new() -> Self {
        let [sclk, mclk, vddgfx, vddnb, temp, power, fan_rpm] = [0; 7]
            .map(|_| History::new(HISTORY_LENGTH, f32::INFINITY));

        Self { sclk, mclk, vddgfx, vddnb, temp, power, fan_rpm }
    }

    pub fn add(&mut self, sec: f64, sensors: &Sensors) {
        for (history, val) in [
            (&mut self.sclk, sensors.sclk),
            (&mut self.mclk, sensors.mclk),
            (&mut self.vddgfx, sensors.vddgfx),
            (&mut self.vddnb, sensors.vddnb),
            (&mut self.temp, sensors.temp.map(|v| v.saturating_div(1000))),
            (&mut self.power, sensors.power),
            (&mut self.fan_rpm, sensors.fan_rpm),
        ] {
            let Some(val) = val else { continue };
            history.add(sec, val);
        }
    }
}

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

pub fn collapsing(
    ui: &mut egui::Ui,
    text: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), Id::new(text), default_open);

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

pub fn rt_base<T: Into<String>>(s: T) -> RichText {
    RichText::new(s.into()).font(BASE)
}
