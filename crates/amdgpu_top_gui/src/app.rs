use std::sync::{Arc, Mutex};
use std::ops::RangeInclusive;
use std::path::PathBuf;
use eframe::egui;
use egui::{RichText, util::History};
use egui::plot::{Corner, Legend, Line, Plot, PlotPoint, PlotPoints};
use crate::{BASE, MEDIUM, HISTORY_LENGTH};

use libamdgpu_top::AMDGPU::{
    MetricsInfo,
    GPU_INFO,
    VBIOS::VbiosInfo,
    VIDEO_CAPS::VideoCapsInfo,
};
use libamdgpu_top::stat::{self, gpu_metrics_util::*, FdInfoSortType, PerfCounter};

use crate::{AppDeviceInfo, CentralData, GpuMetrics, util::*};

const PLOT_HEIGHT: f32 = 32.0;
const PLOT_WIDTH: f32 = 240.0;

pub struct MyApp {
    pub command_path: PathBuf,
    pub app_device_info: AppDeviceInfo,
    pub device_list: Vec<DeviceListMenu>,
    pub decode: Option<VideoCapsInfo>,
    pub encode: Option<VideoCapsInfo>,
    pub vbios: Option<VbiosInfo>,
    pub has_vcn_unified: bool,
    pub support_pcie_bw: bool,
    pub fdinfo_sort: FdInfoSortType,
    pub reverse_sort: bool,
    pub buf_data: CentralData,
    pub arc_data: Arc<Mutex<CentralData>>,
}

impl MyApp {
    pub fn egui_device_list(&self, ui: &mut egui::Ui) {
        ui.menu_button(RichText::new("Device List").font(BASE), |ui| {
            ui.set_width(360.0);
            for device in &self.device_list {
                ui.horizontal(|ui| {
                    let text = RichText::new(format!(
                        "#{instance} {name} ({pci})",
                        instance = device.instance,
                        name = device.name,
                        pci = device.pci,
                    )).font(BASE);

                    if self.app_device_info.pci_bus == device.pci {
                        ui.add_enabled(false, egui::Button::new(text));
                    } else {
                        ui.menu_button(text, |ui| {
                            if ui.button("Launch in a new process").clicked() {
                                std::process::Command::new(&self.command_path)
                                    .args(["--gui", "--pci", &device.pci.to_string()])
                                    .spawn()
                                    .unwrap();
                            }
                        });
                    }
                });
            }
        });
    }

    pub fn egui_app_device_info(&self, ui: &mut egui::Ui) {
        egui::Grid::new("app_device_info").show(ui, |ui| {
            let ext_info = &self.app_device_info.ext_info;
            let memory_info = &self.app_device_info.memory_info;
            let pci_bus = &self.app_device_info.pci_bus;
            let (min_gpu_clk, max_gpu_clk) = (
                &self.app_device_info.min_gpu_clk,
                &self.app_device_info.max_gpu_clk,
            );
            let (min_mem_clk, max_mem_clk) = (
                &self.app_device_info.min_mem_clk,
                &self.app_device_info.max_mem_clk,
            );

            let dev_id = format!("{:#0X}.{:#0X}", ext_info.device_id(), ext_info.pci_rev_id());
            let gpu_type = if ext_info.is_apu() { "APU" } else { "dGPU" };
            let family = ext_info.get_family_name();
            let asic = ext_info.get_asic_name();
            let chip_class = ext_info.get_chip_class();

            let grid = |ui: &mut egui::Ui, v: &[(&str, &str)]| {
                for (name, val) in v {
                    ui.label(*name);
                    ui.label(*val);
                    ui.end_row();
                }
            };

            grid(ui, &[
                ("Device Name", &self.app_device_info.marketing_name),
                ("PCI (domain:bus:dev.func)", &pci_bus.to_string()),
                ("DeviceID.RevID", &dev_id),
                ("GPU Type", gpu_type),
                ("Family", &family.to_string()),
                ("ASIC Name", &asic.to_string()),
                ("Chip Class", &chip_class.to_string()),
            ]);
            ui.end_row();

            let max_good_cu_per_sa = ext_info.get_max_good_cu_per_sa();
            let min_good_cu_per_sa = ext_info.get_min_good_cu_per_sa();
            let cu_per_sa = if max_good_cu_per_sa != min_good_cu_per_sa {
                format!("[{min_good_cu_per_sa}, {max_good_cu_per_sa}]")
            } else {
                max_good_cu_per_sa.to_string()
            };
            let rb_pipes = ext_info.rb_pipes();
            let rop_count = ext_info.calc_rop_count();
            let rb_type = if asic.rbplus_allowed() {
                "RenderBackendPlus (RB+)"
            } else {
                "RenderBackend (RB)"
            };
            let peak_gp = format!("{} GP/s", rop_count * max_gpu_clk / 1000);
            let peak_fp32 = format!("{} GFLOPS", ext_info.peak_gflops());

            grid(ui, &[
                ("Shader Engine (SE)", &ext_info.max_se().to_string()),
                ("Shader Array (SA/SH) per SE", &ext_info.max_sa_per_se().to_string()),
                ("CU per SA", &cu_per_sa),
                ("Total CU", &ext_info.cu_active_number().to_string()),
                (rb_type, &format!("{rb_pipes} ({rop_count} ROPs)")),
                ("Peak Pixel Fill-Rate", &peak_gp),
                ("GPU Clock", &format!("{min_gpu_clk}-{max_gpu_clk} MHz")),
                ("Peak FP32", &peak_fp32),
            ]);
            ui.end_row();

            let re_bar = if self.app_device_info.resizable_bar {
                "Enabled"
            } else {
                "Disabled"
            };

            grid(ui, &[
                ("VRAM Type", &ext_info.get_vram_type().to_string()),
                ("VRAM Bit Width", &format!("{}-bit", ext_info.vram_bit_width)),
                ("VRAM Size", &format!("{} MiB", memory_info.vram.total_heap_size >> 20)),
                ("Memory Clock", &format!("{min_mem_clk}-{max_mem_clk} MHz")),
                ("ResizableBAR", re_bar),
            ]);
            ui.end_row();

            let gl1_cache_size = ext_info.get_gl1_cache_size() >> 10;
            let l3_cache_size = ext_info.calc_l3_cache_size_mb();

            ui.label("L1 Cache (per CU)");
            ui.label(format!("{:4} KiB", ext_info.get_l1_cache_size() >> 10));
            ui.end_row();
            if 0 < gl1_cache_size {
                ui.label("GL1 Cache (per SA/SH)");
                ui.label(format!("{gl1_cache_size:4} KiB"));
                ui.end_row();
            }
            ui.label("L2 Cache");
            ui.label(format!(
                "{:4} KiB ({} Banks)",
                ext_info.calc_l2_cache_size() >> 10,
                ext_info.num_tcc_blocks
            ));
            ui.end_row();
            if 0 < l3_cache_size {
                ui.label("L3 Cache (MALL)");
                ui.label(format!("{l3_cache_size:4} MiB"));
                ui.end_row();
            }
            ui.end_row();

            if let Some(ref cap) = &self.app_device_info.power_cap {
                ui.label("Power Cap.");
                ui.label(format!("{:4} W ({}-{} W)", cap.current, cap.min, cap.max));
                ui.end_row();
                ui.label("Power Cap. (Default)");
                ui.label(format!("{:4} W", cap.default));
                ui.end_row();
            }

            for temp in [
                &self.app_device_info.edge_temp,
                &self.app_device_info.junction_temp,
                &self.app_device_info.memory_temp,
            ] {
                let Some(temp) = temp else { continue };
                let name = temp.type_.to_string();
                if let Some(crit) = temp.critical {
                    ui.label(format!("{name} Temp. (Critical)"));
                    ui.label(format!("{crit:4} C"));
                    ui.end_row();
                }
                if let Some(e) = temp.emergency {
                    ui.label(format!("{name} Temp. (Emergency)"));
                    ui.label(format!("{e:4} C"));
                    ui.end_row();
                }
            }

            for (label, val, unit) in [
                ("Fan RPM (Max).", &self.app_device_info.fan_max_rpm, "RPM"),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label(format!("{val:4} {unit}"));
                ui.end_row();
            }
        });
    }

    pub fn egui_hw_ip_info(&self, ui: &mut egui::Ui) {
        egui::Grid::new("hw_ip_info").show(ui, |ui| {
            ui.label("IP").highlight();
            ui.label("version").highlight();
            ui.label("queues").highlight();
            ui.end_row();

            for hw_ip in &self.app_device_info.hw_ip_info {
                let (major, minor) = hw_ip.info.version();

                ui.label(hw_ip.ip_type.to_string());
                ui.label(format!("{major}.{minor}"));
                ui.label(hw_ip.info.num_queues().to_string());
                ui.end_row();
            }
        });
    }

    pub fn egui_video_caps_info(&self, ui: &mut egui::Ui) {
        let Some(ref decode_caps) = self.decode else { return };
        let Some(ref encode_caps) = self.encode else { return };

        egui::Grid::new("codec_info").show(ui, |ui| {
            ui.label("Codec").highlight();
            ui.label("Decode").highlight();
            ui.label("Encode").highlight();
            ui.end_row();
            
            for (name, decode, encode) in [
                ("MPEG2", decode_caps.mpeg2, encode_caps.mpeg2),
                ("MPEG4", decode_caps.mpeg4, encode_caps.mpeg4),
                ("VC1", decode_caps.vc1, encode_caps.vc1),
                ("MPEG4_AVC", decode_caps.mpeg4_avc, encode_caps.mpeg4_avc),
                ("HEVC", decode_caps.hevc, encode_caps.hevc),
                ("JPEG", decode_caps.jpeg, encode_caps.jpeg),
                ("VP9", decode_caps.vp9, encode_caps.vp9),
                ("AV1", decode_caps.av1, encode_caps.av1),
            ] {
                ui.label(name);
                if let Some(dec) = decode {
                    ui.label(&format!("{}x{}", dec.max_width, dec.max_height));
                } else {
                    ui.label("N/A");
                }
                if let Some(enc) = encode {
                    ui.label(&format!("{}x{}", enc.max_width, enc.max_height));
                } else {
                    ui.label("N/A");
                }
                ui.end_row();
            }
        });
    }

    pub fn egui_vbios_info(&self, ui: &mut egui::Ui) {
        let Some(ref vbios) = self.vbios else { return };
        egui::Grid::new("vbios_info").show(ui, |ui| {
            for (name, val) in [
                ("Name", &vbios.name),
                ("PN", &vbios.pn),
                ("Version", &vbios.ver),
                ("Date", &vbios.date),
            ] {
                ui.label(name).highlight();
                ui.label(val);
                ui.end_row();
            }
        });
    }

    pub fn egui_perf_counter(
        &self,
        ui: &mut egui::Ui,
        name: &str,
        pc: &PerfCounter,
        history: &[History<u8>],
    ) {
        let label_fmt = |_s: &str, val: &PlotPoint| {
            format!("{:.1}s : {:.0}%", val.x, val.y)
        };

        egui::Grid::new(name).show(ui, |ui| {
            for ((name, pos), history) in pc.index.iter().zip(history.iter()) {
                let usage = pc.bits.get(*pos);
                ui.label(name);
                ui.label(format!("{usage:3}%"));

                let points: PlotPoints = history.iter()
                    .map(|(i, val)| [i, val as f64]).collect();
                let line = Line::new(points).fill(1.0);
                Plot::new(name)
                    .allow_drag(false)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .include_y(0.0)
                    .include_y(100.0)
                    .y_axis_formatter(empty_y_fmt)
                    .label_formatter(label_fmt)
                    .auto_bounds_x()
                    .height(PLOT_HEIGHT)
                    .width(PLOT_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
                ui.end_row();
            }
        });
    }

    pub fn egui_vram(&self, ui: &mut egui::Ui) {
        egui::Grid::new("VRAM").show(ui, |ui| {
            for (v, name) in [
                (&self.buf_data.vram_usage.0.vram, "VRAM"),
                (&self.buf_data.vram_usage.0.cpu_accessible_vram, "CPU-Visible VRAM"),
                (&self.buf_data.vram_usage.0.gtt, "GTT"),
            ] {
                let progress = (v.heap_usage >> 20) as f32 / (v.total_heap_size >> 20) as f32;
                let text = format!("{:5} / {:5} MiB", v.heap_usage >> 20, v.total_heap_size >> 20);
                let bar = egui::ProgressBar::new(progress)
                    .text(RichText::new(&text).font(BASE));
                ui.label(RichText::new(name).font(MEDIUM));
                ui.add_sized([360.0, 16.0], bar);
                ui.end_row();
            }
        });
    }

    fn set_fdinfo_sort_type(&mut self, sort_type: FdInfoSortType) {
        if sort_type == self.fdinfo_sort {
            self.reverse_sort ^= true;
        } else {
            self.reverse_sort = false;
        }
        self.fdinfo_sort = sort_type;
    }

    pub fn egui_fdinfo_plot(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0}%", val.x, val.y)
        };

        let [mut gfx, mut compute, mut dma, mut dec, mut enc] = [0; 5]
            .map(|_| Vec::<[f64; 2]>::with_capacity(HISTORY_LENGTH.end));

        for (i, usage) in self.buf_data.fdinfo_history.iter() {
            let usage_dec = usage.dec + usage.vcn_jpeg;
            let usage_enc = usage.enc + usage.uvd_enc;

            gfx.push([i, usage.gfx as f64]);
            compute.push([i, usage.compute as f64]);
            dma.push([i, usage.dma as f64]);
            dec.push([i, usage_dec as f64]);
            enc.push([i, usage_enc as f64]);
        }

        Plot::new("fdinfo plot")
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .include_y(0.0)
            .include_y(100.0)
            .y_axis_formatter(empty_y_fmt)
            .label_formatter(label_fmt)
            .auto_bounds_x()
            .height(ui.available_width() / 4.0)
            .width(ui.available_width() - 36.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (usage, name) in [
                    (gfx, "GFX"),
                    (compute, "Compute"),
                    (dma, "DMA"),
                    (dec, "Decode"),
                    (enc, "Encode"),
                ] {
                    plot_ui.line(Line::new(PlotPoints::new(usage)).name(name));
                }
            });
    }

    pub fn egui_grid_fdinfo(&mut self, ui: &mut egui::Ui) {
        collapsing_plot(ui, "fdinfo Plot", true, |ui| self.egui_fdinfo_plot(ui));

        egui::Grid::new("fdinfo").show(ui, |ui| {
            ui.style_mut().override_font_id = Some(MEDIUM);
            ui.label(rt_base(format!("{:^15}", "Name"))).highlight();
            ui.label(rt_base(format!("{:^8}", "PID"))).highlight();
            if ui.button(rt_base(format!("{:^10}", "VRAM"))).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::VRAM);
            }
            if ui.button(rt_base(format!("{:^10}", "GTT"))).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::GTT);
            }
            if ui.button(rt_base(" GFX ")).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::GFX);
            }
            if ui.button(rt_base("Compute")).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::Compute);
            }
            if ui.button(rt_base(" DMA ")).clicked() {
                self.set_fdinfo_sort_type(FdInfoSortType::DMA);
            }
            if self.has_vcn_unified {
                if ui.button(rt_base(" VCN ")).clicked() {
                    self.set_fdinfo_sort_type(FdInfoSortType::Encode);
                }
            } else {
                if ui.button(rt_base("Decode")).clicked() {
                    self.set_fdinfo_sort_type(FdInfoSortType::Decode);
                }
                if ui.button(rt_base("Encode")).clicked() {
                    self.set_fdinfo_sort_type(FdInfoSortType::Encode);
                }
            }
            ui.end_row();

            stat::sort_proc_usage(
                &mut self.buf_data.fdinfo.proc_usage,
                &self.fdinfo_sort,
                self.reverse_sort,
            );

            for pu in &self.buf_data.fdinfo.proc_usage {
                ui.label(pu.name.to_string());
                ui.label(format!("{:>8}", pu.pid));
                ui.label(format!("{:5} MiB", pu.usage.vram_usage >> 10));
                ui.label(format!("{:5} MiB", pu.usage.gtt_usage >> 10));
                for usage in [
                    pu.usage.gfx,
                    pu.usage.compute,
                    pu.usage.dma,
                ] {
                    ui.label(format!("{usage:3} %"));
                }

        /*
            From VCN4, the encoding queue and decoding queue have been unified.
            The AMDGPU driver handles both decoding and encoding as contexts for the encoding engine.
        */
                if self.has_vcn_unified {
                    ui.label(format!("{:3} %", pu.usage.enc));
                } else {
                    let dec_usage = pu.usage.dec + pu.usage.vcn_jpeg;
                    let enc_usage = pu.usage.enc + pu.usage.uvd_enc;
                    ui.label(format!("{dec_usage:3} %"));
                    ui.label(format!("{enc_usage:3} %"));
                }
                ui.end_row();
            } // proc_usage
        });
    }

    pub fn egui_sensors(&self, ui: &mut egui::Ui) {
        ui.style_mut().override_font_id = Some(MEDIUM);
        let sensors = &self.buf_data.sensors;
        egui::Grid::new("Sensors").show(ui, |ui| {
            for (history, val, label, min, max, unit) in [
                (
                    &self.buf_data.sensors_history.sclk,
                    sensors.sclk,
                    "GFX_SCLK",
                    self.app_device_info.min_gpu_clk,
                    self.app_device_info.max_gpu_clk,
                    "MHz",
                ),
                (
                    &self.buf_data.sensors_history.mclk,
                    sensors.mclk,
                    "GFX_MCLK",
                    self.app_device_info.min_mem_clk,
                    self.app_device_info.max_mem_clk,
                    "MHz",
                ),
                (
                    &self.buf_data.sensors_history.vddgfx,
                    sensors.vddgfx,
                    "VDDGFX",
                    500, // "500 mV" is not an exact value
                    1500, // "1500 mV" is not an exact value
                    "mV",
                ),
                (
                    &self.buf_data.sensors_history.power,
                    sensors.power,
                    "GFX Power",
                    0,
                    if let Some(ref cap) = sensors.power_cap { cap.current } else { 350 }, // "350 W" is not an exact value
                    "W",
                ),
                (
                    &self.buf_data.sensors_history.fan_rpm,
                    sensors.fan_rpm,
                    "Fan",
                    0,
                    sensors.fan_max_rpm.unwrap_or(6000), // "6000 RPM" is not an exact value
                    "RPM",
                ),
            ] {
                let Some(val) = val else { continue };

                ui.label(format!("{label}\n({val:4} {unit})"));

                if min == max {
                    ui.end_row();
                    continue;
                }

                let label_fmt = move |_name: &str, val: &PlotPoint| {
                    format!("{:.1}s\n{:.0} {unit}", val.x, val.y)
                };
                let points: PlotPoints = history.iter()
                    .map(|(i, val)| [i, val as f64]).collect();
                let line = Line::new(points).fill(1.0);
                Plot::new(label)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .include_y(min)
                    .include_y(max)
                    .y_axis_formatter(empty_y_fmt)
                    .label_formatter(label_fmt)
                    .auto_bounds_x()
                    .height(PLOT_HEIGHT * 1.5)
                    .width(PLOT_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
                ui.end_row();
            }
        });

        self.egui_temp_plot(ui);

        if sensors.has_pcie_dpm {
            ui.label(format!(
                "PCIe Link Speed => Gen{cur_gen}x{cur_width:<2} (Max. Gen{max_gen}x{max_width})",
                cur_gen = sensors.cur.gen,
                cur_width = sensors.cur.width,
                max_gen = sensors.max.gen,
                max_width = sensors.max.width,
            ));
        }
    }

    pub fn egui_temp_plot(&self, ui: &mut egui::Ui) {
        ui.style_mut().override_font_id = Some(MEDIUM);
        let sensors = &self.buf_data.sensors;
        let label_fmt = |_name: &str, val: &PlotPoint| {
            format!("{:.1}s\n{:.0} C", val.x, val.y)
        };

        egui::Grid::new("Temp. Sensors").show(ui, |ui| {
            for (label, temp, temp_history) in [
                ("Edge", &sensors.edge_temp, &self.buf_data.sensors_history.edge_temp),
                ("Junction", &sensors.junction_temp, &self.buf_data.sensors_history.junction_temp),
                ("Memory", &sensors.memory_temp, &self.buf_data.sensors_history.memory_temp),
            ] {
                let Some(temp) = temp else { continue };
                let val = temp.current;
                let max = temp.critical.unwrap_or(105) as f64;

                ui.label(format!("{label} Temp.\n({val:4} C)"));

                let points: PlotPoints = temp_history.iter()
                    .map(|(i, val)| [i, val as f64]).collect();
                let line = Line::new(points).fill(1.0);
                Plot::new(label)
                    .allow_zoom(false)
                    .allow_scroll(false)
                    .include_y(0.0)
                    .include_y(max)
                    .y_axis_formatter(empty_y_fmt)
                    .label_formatter(label_fmt)
                    .auto_bounds_x()
                    .auto_bounds_y()
                    .height(PLOT_HEIGHT * 1.5)
                    .width(PLOT_WIDTH)
                    .show(ui, |plot_ui| plot_ui.line(line));
                ui.end_row();
            }
        });
    }

    pub fn egui_pcie_bw(&self, ui: &mut egui::Ui) {
        let label_fmt = |name: &str, val: &PlotPoint| {
            format!("{:.1}s : {name} {:.0} MiB/s", val.x, val.y)
        };

        let [sent, rec] = {
            let [mut sent_history, mut rec_history] = [0; 2].map(|_| Vec::<[f64; 2]>::new());

            for (i, (sent, rec)) in self.buf_data.pcie_bw_history.iter() {
                sent_history.push([i, sent as f64]);
                rec_history.push([i, rec as f64]);
            }

            [
                Line::new(PlotPoints::new(sent_history)).name("Sent"),
                Line::new(PlotPoints::new(rec_history)).name("Received"),
            ]
        };

        Plot::new("pcie_bw plot")
            .allow_zoom(false)
            .allow_scroll(false)
            .include_y(0.0)
            .y_axis_formatter(empty_y_fmt)
            .label_formatter(label_fmt)
            .auto_bounds_x()
            .auto_bounds_y()
            .height(ui.available_width() / 4.0)
            .width(ui.available_width() - 36.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show(ui, |plot_ui| {
                plot_ui.line(sent);
                plot_ui.line(rec);
            });

        if let Some((sent, rec)) = self.buf_data.pcie_bw_history.latest() {
            ui.label(format!("Sent: {sent:5} MiB/s, Received: {rec:5} MiB/s"));
        } else {
            ui.label("Sent: _ MiB/s, Received: _ MiB/s");
        }
    }

    pub fn egui_gpu_metrics_v1(&self, ui: &mut egui::Ui) {
        let gpu_metrics = &self.buf_data.gpu_metrics;

        socket_power(ui, gpu_metrics);
        avg_activity(ui, gpu_metrics);

        ui.horizontal(|ui| {
            v1_helper(ui, "C", &[
                (gpu_metrics.get_temperature_vrgfx(), "VRGFX"),
                (gpu_metrics.get_temperature_vrsoc(), "VRSOC"),
                (gpu_metrics.get_temperature_vrmem(), "VRMEM"),
            ]);
        });

        ui.horizontal(|ui| {
            v1_helper(ui, "mV", &[
                (gpu_metrics.get_voltage_soc(), "SoC"),
                (gpu_metrics.get_voltage_gfx(), "GFX"),
                (gpu_metrics.get_voltage_mem(), "Mem"),
            ]);
        });

        for (avg, cur, name) in [
            (
                gpu_metrics.get_average_gfxclk_frequency(),
                gpu_metrics.get_current_gfxclk(),
                "GFXCLK",
            ),
            (
                gpu_metrics.get_average_socclk_frequency(),
                gpu_metrics.get_current_socclk(),
                "SOCCLK",
            ),
            (
                gpu_metrics.get_average_uclk_frequency(),
                gpu_metrics.get_current_uclk(),
                "UMCCLK",
            ),
            (
                gpu_metrics.get_average_vclk_frequency(),
                gpu_metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                gpu_metrics.get_average_dclk_frequency(),
                gpu_metrics.get_current_dclk(),
                "DCLK",
            ),
            (
                gpu_metrics.get_average_vclk1_frequency(),
                gpu_metrics.get_current_vclk1(),
                "VCLK1",
            ),
            (
                gpu_metrics.get_average_dclk1_frequency(),
                gpu_metrics.get_current_dclk1(),
                "DCLK1",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz"));
        }

        // Only Aldebaran (MI200) supports it.
        if let Some(hbm_temp) = check_hbm_temp(gpu_metrics.get_temperature_hbm()) {
            ui.horizontal(|ui| {
                ui.label("HBM Temp. (C) => [");
                for v in &hbm_temp {
                    ui.label(RichText::new(format!("{v:>5},")));
                }
                ui.label("]");
            });
        }

        throttle_status(ui, gpu_metrics);
    }

    pub fn egui_gpu_metrics_v2(&self, ui: &mut egui::Ui) {
        const CORE_TEMP_LABEL: &str = "Core Temp (C)";
        const CORE_POWER_LABEL: &str = "Core Power (mW)";
        const CORE_CLOCK_LABEL: &str = "Core Clock (MHz)";
        const L3_TEMP_LABEL: &str = "L3 Cache Temp (C)";
        const L3_CLOCK_LABEL: &str = "L3 Cache Clock (MHz)";

        let gpu_metrics = &self.buf_data.gpu_metrics;

        ui.horizontal(|ui| {
            ui.label("GFX =>");
            let temp_gfx = gpu_metrics.get_temperature_gfx().map(|v| v.saturating_div(100));
            v2_helper(ui, &[
                (temp_gfx, "C"),
                (gpu_metrics.get_average_gfx_power(), "mW"),
                (gpu_metrics.get_current_gfxclk(), "MHz"),
            ]);
        });

        ui.horizontal(|ui| {
            ui.label("SoC =>");
            let temp_soc = gpu_metrics.get_temperature_soc().map(|v| v.saturating_div(100));
            v2_helper(ui, &[
                (temp_soc, "C"),
                (gpu_metrics.get_average_soc_power(), "mW"),
                (gpu_metrics.get_current_socclk(), "MHz"),
            ]);
        });

        socket_power(ui, gpu_metrics);
        avg_activity(ui, gpu_metrics);

        for (avg, cur, name) in [
            (
                gpu_metrics.get_average_uclk_frequency(),
                gpu_metrics.get_current_uclk(),
                "UMCCLK",
            ),
            (
                gpu_metrics.get_average_fclk_frequency(),
                gpu_metrics.get_current_fclk(),
                "FCLK",
            ),
            (
                gpu_metrics.get_average_vclk_frequency(),
                gpu_metrics.get_current_vclk(),
                "VCLK",
            ),
            (
                gpu_metrics.get_average_dclk_frequency(),
                gpu_metrics.get_current_dclk(),
                "DCLK",
            ),
        ] {
            let [avg, cur] = [avg, cur].map(check_metrics_val);
            ui.label(format!("{name:<6} => Avg. {avg:>4} MHz, Cur. {cur:>4} MHz"));
        }

        egui::Grid::new("GPU Metrics v2.x Core/L3").show(ui, |ui| {
            let core_temp = check_temp_array(gpu_metrics.get_temperature_core());
            let l3_temp = check_temp_array(gpu_metrics.get_temperature_l3());
            let [core_power, core_clk] = [
                gpu_metrics.get_average_core_power(),
                gpu_metrics.get_current_coreclk(),
            ].map(check_power_clock_array);
            let l3_clk = check_power_clock_array(gpu_metrics.get_current_l3clk());

            for (val, label) in [
                (core_temp, CORE_TEMP_LABEL),
                (core_power, CORE_POWER_LABEL),
                (core_clk, CORE_CLOCK_LABEL),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label("=> [");
                for v in &val {
                    ui.label(RichText::new(format!("{v:>5},")));
                }
                ui.label("]");
                ui.end_row();
            }

            for (val, label) in [
                (l3_temp, L3_TEMP_LABEL),
                (l3_clk, L3_CLOCK_LABEL),
            ] {
                let Some(val) = val else { continue };
                ui.label(label);
                ui.label("=> [");
                for v in &val {
                    ui.label(RichText::new(format!("{v:>5},")));
                }
                ui.label("]");
                ui.end_row();
            }
        });

        throttle_status(ui, gpu_metrics);
    }
}

fn empty_y_fmt(_y: f64, _range: &RangeInclusive<f64>) -> String {
    String::new()
}

fn socket_power(ui: &mut egui::Ui, gpu_metrics: &GpuMetrics) {
    let v = check_metrics_val(gpu_metrics.get_average_socket_power());
    ui.label(format!("Socket Power => {v:>3} W"));
}

fn avg_activity(ui: &mut egui::Ui, gpu_metrics: &GpuMetrics) {
    ui.horizontal(|ui| {
        ui.label("Average Activity =>");
        for (val, label) in [
            (gpu_metrics.get_average_gfx_activity(), "GFX"),
            (gpu_metrics.get_average_umc_activity(), "UMC"),
            (gpu_metrics.get_average_mm_activity(), "Media"),
        ] {
            let v = check_metrics_val(val.map(|v| v.saturating_div(100)));
            ui.label(format!("{label} {v:>3}%,"));
        }
    });
}

fn throttle_status(ui: &mut egui::Ui, gpu_metrics: &GpuMetrics) {
    if let Some(throttle) = gpu_metrics.get_throttle_status() {
        let thr = format!("{throttle:032b}");
        ui.label(
            format!(
                "Throttle Status: {}_{}_{}_{}",
                &thr[..8],
                &thr[8..16],
                &thr[16..24],
                &thr[24..32],
            )
        );
    }
}

fn v1_helper(ui: &mut egui::Ui, unit: &str, v: &[(Option<u16>, &str)]) {
    for (val, name) in v {
        let v = check_metrics_val(*val);
        ui.label(format!("{name} => {v:>4} {unit}, "));
    }
}

fn v2_helper(ui: &mut egui::Ui, v: &[(Option<u16>, &str)]) {
    for (val, unit) in v {
        let v = check_metrics_val(*val);
        ui.label(format!("{v:>5} {unit}, "));
    }
}
