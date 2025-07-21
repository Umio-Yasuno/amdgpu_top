use std::sync::{Arc, Mutex};
use eframe::wgpu::AdapterInfo;
use crate::egui::{self, RichText, util::History};
use crate::{BASE, MEDIUM, HISTORY_LENGTH};
use crate::{GuiAppData, GuiGpuMetrics, util::*, fl};
use crate::tab_gui::{MainTab, InfoTab};
use egui_plot::{Corner, Legend, Line, Plot, PlotPoint, PlotPoints};

use libamdgpu_top::{
    AMDGPU::{GpuMetrics, MetricsInfo, RasErrorCount},
    DevicePath,
    PCI,
    stat::{FdInfoSortType, PerfCounter, Sensors},
};

const SPACING: [f32; 2] = [16.0; 2];

const SENSORS_HEIGHT: f32 = 96.0;
const SENSORS_WIDTH: f32 = SENSORS_HEIGHT * 4.0;
const FDINFO_LIST_HEIGHT: f32 = 208.0;
const PLOT_HEIGHT: f32 = 208.0;
const PLOT_WIDTH: f32 = PLOT_HEIGHT * 5.0;

pub struct MyApp {
    pub fdinfo_sort: FdInfoSortType,
    pub reverse_sort: bool,
    pub buf_data: GuiAppData,
    pub buf_vec_data: Vec<GuiAppData>,
    pub arc_data: Arc<Mutex<Vec<GuiAppData>>>,
    pub device_path_list: Vec<DevicePath>,
    pub show_sidepanel: bool,
    pub wgpu_adapter_info: Option<AdapterInfo>,
    pub rocm_version: Option<String>,
    pub selected_pci_bus: PCI::BUS_INFO,
    pub no_pc: bool,
    pub pause: bool,
    pub full_fdinfo_list: bool,
    pub tab_gui: bool,
    pub main_tab: MainTab,
    pub info_tab: InfoTab,
}

pub fn grid(ui: &mut egui::Ui, v: &[(&str, &str)]) {
    for (name, val) in v {
        ui.label(*name);
        ui.label(*val);
        ui.end_row();
    }
}

pub trait GuiMemoryErrorCount {
    fn ui(&self, ui: &mut egui::Ui);
}

impl GuiMemoryErrorCount for RasErrorCount {
    fn ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("ECC Memory Error Count").show(ui, |ui| {
            ui.label(fl!("corrected"));
            ui.label(self.corrected.to_string());
            ui.end_row();

            ui.label(fl!("uncorrected"));
            ui.label(self.uncorrected.to_string());
            ui.end_row();
        });
    }
}

impl MyApp {
    pub fn egui_perf_counter(
        &self,
        ui: &mut egui::Ui,
        pc_name: &str,
        pc: &PerfCounter,
        history: &[History<u8>],
    ) {
        let label_fmt = |_s: &str, val: &PlotPoint| {
            format!("{:.1}s : {:.0}%", val.x, val.y)
        };
        let mut n = 1;

        egui::Grid::new(pc_name).spacing(SPACING).show(ui, |ui| {
            for (pc_index, history) in pc.pc_index.iter().zip(history.iter()) {
                egui::Grid::new(&pc_index.name).show(ui, |ui| {
                    let usage = pc_index.usage;
                    ui.label(format!("{} {usage:3}%", &pc_index.name));
                    ui.end_row();

                    let points: PlotPoints = history.iter()
                        .map(|(i, val)| [i, val as f64]).collect();
                    let line = Line::new(pc_index.name.clone(), points).fill(0.0);

                    default_plot(&pc_index.name)
                        .allow_scroll(false)
                        .include_y(0.0)
                        .include_y(100.0)
                        .label_formatter(label_fmt)
                        .auto_bounds([true, false])
                        .height(SENSORS_HEIGHT / 2.0)
                        .width(SENSORS_WIDTH)
                        .show(ui, |plot_ui| plot_ui.line(line));
                });

                n += 1;
                if n % 2 == 1 { ui.end_row(); }
            }
        });
    }

    pub fn egui_vram_plot(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} MiB", val.x, val.y)
        };

        let [vram, gtt] = [
            &self.buf_data.history.vram_history,
            &self.buf_data.history.gtt_history,
        ].map(|history| {
            history.iter().map(|(i, usage)| [i, (usage >> 20) as f64]).collect()
        });

        let max = std::cmp::max(
          self.buf_data.stat.vram_usage.0.vram.total_heap_size >> 20,
          self.buf_data.stat.vram_usage.0.gtt.total_heap_size >> 20,
        );

        default_plot("VRAM Plot")
            .allow_scroll(false)
            .include_y(max as f64)
            .label_formatter(label_fmt)
            .auto_bounds([true, false])
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width()))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (usage, name) in [
                    (vram, fl!("vram")),
                    (gtt, fl!("gtt"))
                ] {
                    plot_ui.line(Line::new(name, PlotPoints::new(usage)));
                }
            });
    }

    pub fn egui_vram(&self, ui: &mut egui::Ui) {
        collapsing_plot(ui, &fl!("vram_plot"), true, |ui| self.egui_vram_plot(ui));

        egui::Grid::new("VRAM").show(ui, |ui| {
            let mib = fl!("mib");
            for (v, name) in [
                (&self.buf_data.stat.vram_usage.0.vram, fl!("vram")),
                (&self.buf_data.stat.vram_usage.0.cpu_accessible_vram, fl!("cpu_visible_vram")),
                (&self.buf_data.stat.vram_usage.0.gtt, fl!("gtt")),
            ] {
                let progress = (v.heap_usage >> 20) as f32 / (v.total_heap_size >> 20) as f32;
                let text = format!("{:5} / {:5} ({}: {:5}) {mib}", v.heap_usage >> 20, v.total_heap_size >> 20, fl!("usable"), v.usable_heap_size >> 20);
                let bar = egui::ProgressBar::new(progress)
                    .text(RichText::new(&text).font(BASE));
                ui.label(RichText::new(name).font(MEDIUM));
                ui.add_sized([360.0, 16.0], bar);
                ui.end_row();
            }
        });
    }

    pub fn egui_fdinfo_plot(&self, ui: &mut egui::Ui, has_vcn_unified: bool, has_vpe: bool) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0}%", val.x, val.y)
        };

        let [mut gfx, mut compute, mut dma, mut dec, mut enc, mut vcnu, mut vpe] = [0; 7]
            .map(|_| Vec::<[f64; 2]>::with_capacity(HISTORY_LENGTH.end));

        for (i, usage) in self.buf_data.history.fdinfo_history.iter() {
            gfx.push([i, usage.gfx as f64]);
            compute.push([i, usage.compute as f64]);
            dma.push([i, usage.dma as f64]);

            if has_vcn_unified {
                vcnu.push([i, usage.vcn_unified as f64]);
            } else {
                dec.push([i, usage.total_dec as f64]);
                enc.push([i, usage.total_enc as f64]);
            }

            if has_vpe {
                vpe.push([i, usage.vpe as f64]);
            }
        }

        default_plot("fdinfo Plot")
            .allow_scroll(false)
            .include_y(100.0)
            .show_axes([false, true])
            .label_formatter(label_fmt)
            .auto_bounds([true, false])
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width()))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (usage, name) in [
                    (gfx, fl!("gfx")),
                    (compute, fl!("compute")),
                    (dma, fl!("dma")),
                ] {
                    plot_ui.line(Line::new(name, PlotPoints::new(usage)));
                }

                if has_vcn_unified {
                    plot_ui.line(Line::new(fl!("vcn_unified"), PlotPoints::new(vcnu)));
                } else {
                    plot_ui.line(Line::new(fl!("decode"), PlotPoints::new(dec)));
                    plot_ui.line(Line::new(fl!("encode"), PlotPoints::new(enc)));
                }

                if has_vpe {
                    plot_ui.line(Line::new(fl!("vpe"), PlotPoints::new(vpe)));
                }
            });
    }

    pub fn egui_fdinfo_list(&mut self, ui: &mut egui::Ui, has_vcn_unified: bool, has_vpe: bool) {
        egui::Grid::new("fdinfo").show(ui, |ui| {
            ui.style_mut().override_font_id = Some(MEDIUM);
            ui.label(rt_base(format!("{:^15}", fl!("name")))).highlight();

            for (s, align, sort_type, flag) in [
                (fl!("pid"), 8, FdInfoSortType::PID, true),
                ("KFD".to_string(), 3, FdInfoSortType::KFD, true),
                (fl!("vram"), 10, FdInfoSortType::VRAM, true),
                (fl!("gtt"), 10, FdInfoSortType::GTT, true),
                (fl!("cpu"), 5, FdInfoSortType::CPU, true),
                (fl!("gfx"), 5, FdInfoSortType::GFX, true),
                (fl!("compute"), 9, FdInfoSortType::Compute, true),
                (fl!("dma"), 5, FdInfoSortType::DMA, true),
                (fl!("vcn_unified"), 11, FdInfoSortType::VCNU, has_vcn_unified),
                (fl!("decode"), 9, FdInfoSortType::Decode, !has_vcn_unified),
                (fl!("encode"), 9, FdInfoSortType::Encode, !has_vcn_unified),
                (fl!("vpe"), 5, FdInfoSortType::VPE, has_vpe),
            ] {
                if !flag { continue; }

                let (mark, rev) = match (self.fdinfo_sort == sort_type, self.reverse_sort) {
                    (true, false) => ("▽ ", true),
                    (true, true) => ("△ ", false),
                    _ => ("", false),
                };
                let s = format!("{mark}{s}");
                let s = format!("{s:^align$}");
                if ui.button(rt_base(s)).clicked() {
                    self.reverse_sort = rev;
                    self.fdinfo_sort = sort_type;
                }
            }

            ui.end_row();

            self.buf_data.stat.fdinfo.sort_proc_usage(self.fdinfo_sort, self.reverse_sort);

            let mib = fl!("mib");

            for pu in &self.buf_data.stat.fdinfo.proc_usage {
                if pu.ids_count == 0 { continue; }

                ui.label(pu.name.to_string());
                ui.label(format!("{:>8}", pu.pid));
                ui.label(if pu.is_kfd_process { " Y " } else { "" });
                ui.label(format!("{:5} {mib}", pu.usage.vram_usage >> 10));
                ui.label(format!("{:5} {mib}", pu.usage.gtt_usage >> 10));
                for usage in [
                    pu.usage.cpu,
                    pu.usage.gfx,
                    pu.usage.compute,
                    pu.usage.dma,
                ] {
                    ui.label(format!("{usage:3} %"));
                }

                if has_vcn_unified {
                    ui.label(format!("{:3} %", pu.usage.vcn_unified));
                } else {
                    ui.label(format!("{:3} %", pu.usage.total_dec));
                    ui.label(format!("{:3} %", pu.usage.total_enc));
                }

                if has_vpe {
                    ui.label(format!("{:3} %", pu.usage.vpe));
                }

                ui.end_row();
            } // proc_usage
        });
    }

    pub fn egui_grid_xdna_fdinfo(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("fdinfo").show(ui, |ui| {
            ui.style_mut().override_font_id = Some(MEDIUM);
            ui.label(rt_base(format!("{:^15}", fl!("name")))).highlight();

            for (s, align) in [
                (fl!("pid"), 8),
                (fl!("memory"), 10),
                (fl!("npu"), 5),
            ] {
                let s = format!("{s:^align$}");
                let _ = ui.button(rt_base(s));
            }

            ui.end_row();

            let mib = fl!("mib");

            for pu in &self.buf_data.stat.xdna_fdinfo.proc_usage {
                if pu.ids_count == 0 { continue; }

                ui.label(pu.name.to_string());
                ui.label(format!("{:>8}", pu.pid));
                ui.label(format!("{:5} {mib}", pu.usage.total_memory >> 10));
                ui.label(format!("{:3} %", pu.usage.npu));
                ui.end_row();
            }
        });
    }

    pub fn egui_grid_fdinfo(&mut self, ui: &mut egui::Ui) {
        let has_vcn_unified = self.buf_data.stat.fdinfo.has_vcn_unified;
        let has_vpe = self.buf_data.stat.fdinfo.has_vpe;
        let proc_len = self.buf_data.stat.fdinfo.proc_usage.len();

        collapsing_plot(
            ui,
            &fl!("fdinfo_plot"),
            true,
            |ui| self.egui_fdinfo_plot(ui, has_vcn_unified, has_vpe),
        );

        ui.checkbox(&mut self.full_fdinfo_list, fl!("full_fdinfo_list"));

        if self.full_fdinfo_list || (proc_len != 0 && proc_len < 8) {
            self.egui_fdinfo_list(ui, has_vcn_unified, has_vpe);
        } else {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .min_scrolled_height(FDINFO_LIST_HEIGHT)
                .show(ui, |ui| self.egui_fdinfo_list(ui, has_vcn_unified, has_vpe));
        }
    }

    pub fn egui_sensors(&self, ui: &mut egui::Ui) {
        let Some(sensors) = self.buf_data.stat.sensors.as_ref() else { return };
        let mut n = 1;
        ui.style_mut().override_font_id = Some(MEDIUM);

        egui::Grid::new("Sensors").spacing(SPACING).show(ui, |ui| {
            for (history, val, label, min, max, unit) in [
                (
                    &self.buf_data.history.sensors_history.sclk,
                    sensors.sclk,
                    "GFX_SCLK",
                    // some AMD GPUs support DS (Deep Sleep) state
                    0,
                    self.buf_data.device_info.max_gpu_clk,
                    fl!("mhz"),
                ),
                (
                    &self.buf_data.history.sensors_history.mclk,
                    sensors.mclk,
                    "GFX_MCLK",
                    // some AMD GPUs support DS (Deep Sleep) state
                    0,
                    self.buf_data.device_info.max_mem_clk,
                    fl!("mhz"),
                ),
                (
                    &self.buf_data.history.sensors_history.fclk,
                    sensors.fclk_dpm.as_ref().map(|f| f.current_mhz),
                    "FCLK",
                    sensors.fclk_dpm.as_ref().map(|f| f.min_mhz).unwrap_or(0),
                    sensors.fclk_dpm.as_ref().map(|f| f.max_mhz).unwrap_or(3000),
                    fl!("mhz"),
                ),
                (
                    &self.buf_data.history.sensors_history.vddgfx,
                    sensors.vddgfx,
                    "VDDGFX",
                    500, // "500 mV" is not an exact value
                    1500, // "1500 mV" is not an exact value
                    fl!("mv"),
                ),
                (
                    &self.buf_data.history.sensors_history.average_power,
                    sensors.average_power.as_ref().map(|power| power.value),
                    "Average Power",
                    0,
                    if let Some(cap) = &sensors.power_cap { cap.current } else { 350 }, // "350 W" is not an exact value
                    fl!("w"),
                ),
                (
                    &self.buf_data.history.sensors_history.input_power,
                    sensors.input_power.as_ref().map(|power| power.value),
                    "Input Power",
                    0,
                    if let Some(cap) = &sensors.power_cap { cap.current } else { 350 }, // "350 W" is not an exact value
                    fl!("w"),
                ),
                (
                    &self.buf_data.history.sensors_history.fan_rpm,
                    sensors.fan_rpm,
                    "Fan",
                    0,
                    sensors.fan_max_rpm.unwrap_or(6000), // "6000 RPM" is not an exact value
                    fl!("rpm"),
                ),
            ] {
                let Some(val) = val else { continue };
                let per = if min == 0 {
                    val.saturating_mul(100).checked_div(max)
                } else {
                    None
                };

                egui::Grid::new(label).show(ui, |ui| {
                    if let Some(per) = per {
                        ui.label(format!("{label} ({val:4} {unit}) ({per:>3}%)"));
                    } else {
                        ui.label(format!("{label} ({val:4} {unit})"));
                    }

                    ui.end_row();

                    let label_fmt = move |_name: &str, val: &PlotPoint| {
                        if let Some(per) = per {
                            format!("{:.1}s\n{:.0} {unit} ({per:>3}%)", val.x, val.y)
                        } else {
                            format!("{:.1}s\n{:.0} {unit}", val.x, val.y)
                        }
                    };
                    let points: PlotPoints = history.iter()
                        .map(|(i, val)| [i, val as f64]).collect();
                    let line = Line::new(label.to_string(), points).fill(0.0);

                    Plot::new(label)
                        .allow_zoom(false)
                        .allow_scroll(false)
                        .include_y(min)
                        .include_y(max)
                        .show_axes(false)
                        .label_formatter(label_fmt)
                        .auto_bounds([true, false])
                        .height(SENSORS_HEIGHT)
                        .width(SENSORS_WIDTH)
                        .show(ui, |plot_ui| plot_ui.line(line));
                });

                n += 1;
                if n % 2 == 1 { ui.end_row(); }
            }

            if n % 2 == 0 { ui.end_row(); }
            self.egui_temp_plot(ui);
        });

        self.egui_core_freq_plot(ui, sensors);

        if let Some(cur) = sensors.current_link {
            let min_max = if let [Some(min), Some(max)] = [sensors.min_dpm_link, sensors.max_dpm_link] {
                format!(
                    " (Gen{}x{} - Gen{}x{})",
                    min.r#gen,
                    min.width,
                    max.r#gen,
                    max.width,
                )
            } else if let Some(max) = sensors.max_dpm_link {
                format!(" ({} Gen{}x{})", fl!("max"), max.r#gen, max.width)
            } else {
                String::new()
            };

            ui.label(format!(
                "{} => Gen{}x{} {min_max}",
                fl!("pcie_link_speed"),
                cur.r#gen,
                cur.width,
            ));
        }

        if let Some(power_state) = &sensors.pci_power_state {
            ui.label(format!(
                "{}: {}",
                fl!("pci_power_state"),
                power_state,
            ));
        }

        if let Some(power_profile) = &sensors.power_profile {
            ui.label(format!(
                "{}: {}",
                fl!("power_profile"),
                power_profile,
            ));
        }
    }

    pub fn egui_temp_plot(&self, ui: &mut egui::Ui) {
        let Some(sensors) = self.buf_data.stat.sensors.as_ref() else { return };
        let label_fmt = |_name: &str, val: &PlotPoint| {
            format!("{:.1}s\n{:.0} C", val.x, val.y)
        };
        ui.style_mut().override_font_id = Some(MEDIUM);
        let mut n = 1;

        for (label, temp, temp_history) in [
            ("Edge", &sensors.edge_temp, &self.buf_data.history.sensors_history.edge_temp),
            ("Junction", &sensors.junction_temp, &self.buf_data.history.sensors_history.junction_temp),
            ("Memory", &sensors.memory_temp, &self.buf_data.history.sensors_history.memory_temp),
        ] {
            let Some(temp) = temp else { continue };

            egui::Grid::new(label).show(ui, |ui| {
                let val = temp.current;
                let max = temp.critical.unwrap_or(105) as f64;

                ui.label(format!("{label} Temp. ({val:4} C)"));
                ui.end_row();

                let points: PlotPoints = temp_history
                    .iter()
                    .map(|(i, val)| [i, val as f64])
                    .collect();
                let line = Line::new(label.to_string(), points).fill(0.0);

                default_plot(label)
                    .include_y(max)
                    .label_formatter(label_fmt)
                    .auto_bounds([true, true])
                    .height(SENSORS_HEIGHT)
                    .width(SENSORS_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
            });

            n += 1;
            if n % 2 == 1 { ui.end_row(); }
        }

        if let Some(ref tctl) = sensors.tctl {
            let label = "CPU Tctl";
            egui::Grid::new(label).show(ui, |ui| {
                ui.label(format!("CPU Tctl ({:3} C)", tctl / 1000));
                ui.end_row();

                let points: PlotPoints = self.buf_data.history.sensors_history.tctl
                    .iter()
                    .map(|(i, val)| [i, val as f64])
                    .collect();
                let line = Line::new(label.to_string(), points).fill(0.0);

                default_plot(label)
                    .include_y(0)
                    .include_y(100)
                    .label_formatter(label_fmt)
                    .auto_bounds([true, true])
                    .height(SENSORS_HEIGHT)
                    .width(SENSORS_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
            });

            n += 1;
            if n % 2 == 1 { ui.end_row(); }
        }
    }

    pub fn egui_pcie_bw(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} {}", val.x, val.y, fl!("mib_s"))
        };

        let fl_sent = fl!("sent");
        let fl_rec = fl!("received");
        let mib_s = fl!("mib_s");

        let [sent, rec] = {
            let [mut sent_history, mut rec_history] = [0; 2].map(|_| Vec::<[f64; 2]>::new());

            for (i, (sent, rec)) in self.buf_data.history.pcie_bw_history.iter() {
                sent_history.push([i, sent as f64]);
                rec_history.push([i, rec as f64]);
            }

            [
                Line::new(fl_sent.clone(), PlotPoints::new(sent_history)),
                Line::new(fl_rec.clone(), PlotPoints::new(rec_history)),
            ]
        };

        default_plot("pcie_bw plot")
            .label_formatter(label_fmt)
            .auto_bounds([true, true])
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width()))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(sent);
                plot_ui.line(rec);
            });

        if let Some((sent, rec)) = self.buf_data.history.pcie_bw_history.latest() {
            ui.label(format!("{fl_sent}: {sent:5} {mib_s}, {fl_rec}: {rec:5} {mib_s}"));
        } else {
            ui.label(format!("{fl_sent}: _ {mib_s}, {fl_rec}: _ {mib_s}"));
        }
    }

    pub fn egui_activity(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0}%", val.x, val.y)
        };
        let fl_gfx = fl!("gfx");
        let fl_memory = fl!("memory");
        let fl_media = fl!("media");

        ui.label(format!("{fl_gfx}: {:>3}%, {fl_memory}: {:>3}%, {fl_media}: {:>3}%",
            self.buf_data.stat.activity.gfx.map(|v| v.to_string()).unwrap_or("___".to_string()),
            self.buf_data.stat.activity.umc.map(|v| v.to_string()).unwrap_or("___".to_string()),
            self.buf_data.stat.activity.media.map(|v| v.to_string()).unwrap_or("___".to_string()),
        ));

        let [gfx, umc, media] = [
            (fl_gfx, &self.buf_data.history.gfx_activity),
            (fl_memory, &self.buf_data.history.umc_activity),
            (fl_media, &self.buf_data.history.media_activity),
        ].map(|(name, history)| {
            let v: Vec<_> = history
                .iter()
                .map(|(i, act)| [i, act as f64])
                .collect();

            Line::new(name, PlotPoints::new(v))
        });

        default_plot("activity plot")
            .allow_scroll(false)
            .include_y(0.0)
            .include_y(100.0)
            .label_formatter(label_fmt)
            .show_axes([false, true])
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width()))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(gfx);
                plot_ui.line(umc);
                plot_ui.line(media);
            });
    }

    pub fn egui_core_freq_plot(&self, ui: &mut egui::Ui, sensors: &Sensors) {
        if sensors.all_cpu_core_freq_info.is_empty() {
            return;
        }

        {
            let mut s = String::with_capacity(128);
            let Ok(_) = sensors.print_all_cpu_core_cur_freq(
                &mut s,
                "\nCPU Core freq (MHz)",
                false,
            ) else { return };
            ui.label(s);
            ui.end_row();
        }

        let all_core_freq: Vec<Vec<[f64; 2]>> = self.buf_data.history.sensors_history.core_freq
            .iter()
            .map(|history| history.iter().map(|(i, mhz)| [i, mhz as f64]).collect())
            .collect();
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} MHz", val.x, val.y)
        };

        Plot::new("Core Freq Plot")
            .allow_zoom(false)
            .allow_scroll(false)
            .show_axes([false, true])
            .label_formatter(label_fmt)
            .auto_bounds([true, true])
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width() - 100.0))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| for (i, freq) in all_core_freq.into_iter().enumerate() {
                plot_ui.line(Line::new(format!("Core{i}"), PlotPoints::new(freq)))
            });
        ui.label(""); // \n
    }

    pub fn egui_core_power_plot(&self, ui: &mut egui::Ui) {
        let Some(core_power_mw) = &self.buf_data.history.core_power_mw else { return };
        let all_core_power_mw: Vec<Vec<[f64; 2]>> = core_power_mw
            .iter()
            .map(|history| history.iter().map(|(i, mw)| [i, mw as f64]).collect())
            .collect();
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} mW", val.x, val.y)
        };

        Plot::new("Core Power Plot")
            .allow_zoom(false)
            .allow_scroll(false)
            .show_axes([false, true])
            .label_formatter(label_fmt)
            .auto_bounds([true, true])
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width() - 100.0))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| for (i, mw) in all_core_power_mw.into_iter().enumerate() {
                plot_ui.line(Line::new(format!("Core{i}"), PlotPoints::new(mw)))
            });
    }

    pub fn egui_core_temp_plot(&self, ui: &mut egui::Ui) {
        let Some(core_temp) = &self.buf_data.history.core_temp else { return };
        let all_core_temp: Vec<Vec<[f64; 2]>> = core_temp
            .iter()
            .map(|history| history.iter().map(|(i, mw)| [i, mw as f64]).collect())
            .collect();
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} C", val.x, val.y)
        };

        Plot::new("Core Temperature Plot")
            .allow_zoom(false)
            .allow_scroll(false)
            .show_axes([false, true])
            .include_y(0.0)
            .label_formatter(label_fmt)
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width() - 100.0))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| for (i, temp_c) in all_core_temp.into_iter().enumerate() {
                plot_ui.line(Line::new(format!("Core{i}"), PlotPoints::new(temp_c)))
            });
    }

    pub fn egui_vclk_dclk_plot(&self, ui: &mut egui::Ui) {
        let [vclk, dclk, vclk1, dclk1] = [
            &self.buf_data.history.vclk,
            &self.buf_data.history.dclk,
            &self.buf_data.history.vclk1,
            &self.buf_data.history.dclk1,
        ].map(|history| history.iter().map(|(i, clk)| [i, clk as f64]).collect::<Vec<[f64; 2]>>());
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} MHz", val.x, val.y)
        };

        Plot::new("VCLK/DCLK Plot")
            .allow_zoom(false)
            .allow_scroll(false)
            .show_axes([false, true])
            .include_y(0.0)
            .label_formatter(label_fmt)
            .height(PLOT_HEIGHT)
            .width(PLOT_WIDTH.min(ui.available_width() - 100.0))
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (clk, name) in [
                    (vclk, "VCLK"),
                    (dclk, "DCLK"),
                    (vclk1, "VCLK1"),
                    (dclk1, "DCLK1"),
                ] {
                    if !clk.is_empty() {
                        plot_ui.line(Line::new(name, PlotPoints::new(clk)))
                    }
                }
            });
    }

    pub fn egui_gpu_metrics(&self, ui: &mut egui::Ui) {
        let Some(metrics) = &self.buf_data.stat.metrics else { return };
        let Some(header) = metrics.get_header() else { return };

        let header = format!(
            "{} v{}.{}",
            fl!("gpu_metrics"),
            header.format_revision,
            header.content_revision
        );

        match metrics {
            GpuMetrics::V1_0(_) |
            GpuMetrics::V1_1(_) |
            GpuMetrics::V1_2(_) |
            GpuMetrics::V1_3(_) |
            GpuMetrics::V1_4(_) |
            GpuMetrics::V1_5(_) => {
                collapsing(ui, &header, true, |ui| metrics.v1_ui(ui));
                collapsing_plot(ui, &fl!("vclk_dclk_plot"), true, |ui| self.egui_vclk_dclk_plot(ui));
            },
            /* APU */
            GpuMetrics::V2_0(_) |
            GpuMetrics::V2_1(_) |
            GpuMetrics::V2_2(_) |
            GpuMetrics::V2_3(_) |
            GpuMetrics::V2_4(_) => {
                collapsing(ui, &header, true, |ui| {
                    metrics.v2_ui(ui);
                    if self.buf_data.history.core_temp.is_some() {
                        collapsing_plot(
                            ui,
                            &fl!("cpu_temp_plot"),
                            true,
                            |ui| self.egui_core_temp_plot(ui),
                        );
                    }
                    if self.buf_data.history.core_power_mw.is_some() {
                        collapsing_plot(
                            ui,
                            &fl!("cpu_power_plot"),
                            true,
                            |ui| self.egui_core_power_plot(ui),
                        );
                    }
                    collapsing_plot(ui, &fl!("vclk_dclk_plot"), true, |ui| self.egui_vclk_dclk_plot(ui));
                });
            },
            /* APU */
            GpuMetrics::V3_0(_) => {
                collapsing(ui, &header, true, |ui| {
                    metrics.v3_ui(ui);
                    collapsing_plot(ui, &fl!("cpu_temp_plot"), true, |ui| self.egui_core_temp_plot(ui));
                    collapsing_plot(ui, &fl!("cpu_power_plot"), true, |ui| self.egui_core_power_plot(ui));
                    collapsing_plot(ui, &fl!("vclk_dclk_plot"), true, |ui| self.egui_vclk_dclk_plot(ui));
                });
            },
            _ => {},
        }
    }
}

fn default_plot(id: &str) -> Plot {
    Plot::new(id)
        .allow_zoom(false)
        .allow_scroll(false)
        .include_y(0.0)
        .show_axes(false)
}
