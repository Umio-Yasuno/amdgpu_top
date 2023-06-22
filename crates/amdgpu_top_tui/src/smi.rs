use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::PathBuf;
use cursive::align::HAlign;
use cursive::view::{Nameable, Scrollable};
use cursive::views::{HideableView, LinearLayout, TextContent, TextView, Panel};

use libamdgpu_top::AMDGPU::{DeviceHandle, FAMILY_NAME, GPU_INFO, MetricsInfo};
use libamdgpu_top::{stat, DevicePath, PCI, Sampling, VramUsage};
use stat::{GfxoffStatus, GpuActivity, Sensors, ProcInfo};

use crate::{FdInfoView, Text, ToggleOptions, stat::FdInfoSortType};

const GPU_NAME_LEN: usize = 25;
const LINE_LEN: usize = 150;
const THR_LEN: usize = 60;
const PROC_TITLE: &str = "Processes";

pub(crate) struct SmiDeviceInfo {
    pub amdgpu_dev: DeviceHandle,
    pub device_path: DevicePath,
    pub instance: u32,
    pub marketing_name: String,
    pub pci_bus: PCI::BUS_INFO,
    pub sysfs_path: PathBuf,
    pub cu_number: u32,
    pub vram_usage: VramUsage,
    pub sensors: Sensors,
    pub check_gfxoff: bool,
    pub family_name: FAMILY_NAME,
    pub fdinfo: FdInfoView,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub info_text: Text,
}

impl SmiDeviceInfo {
    pub fn new(amdgpu_dev: DeviceHandle, device_path: &DevicePath, instance: u32) -> Self {
        let marketing_name = amdgpu_dev.get_marketing_name_or_default();
        let pci_bus = match device_path.pci {
            Some(pci_bus) => pci_bus,
            None => amdgpu_dev.get_pci_bus_info().unwrap(),
        };
        let sysfs_path = pci_bus.get_sysfs_path();
        let ext_info = amdgpu_dev.device_info().unwrap();
        let cu_number = ext_info.cu_active_number();
        let memory_info = amdgpu_dev.memory_info().unwrap();
        let vram_usage = VramUsage(memory_info);
        let sensors = Sensors::new(&amdgpu_dev, &pci_bus);
        let check_gfxoff = GfxoffStatus::get(instance).is_ok();
        let family_name = ext_info.get_family_name();

        let mut fdinfo = FdInfoView::new(
            Sampling::default().to_duration(),
            libamdgpu_top::has_vcn_unified(&amdgpu_dev),
        );

        let arc_proc_index = {
            let mut proc_index: Vec<stat::ProcInfo> = Vec::new();
            stat::update_index(&mut proc_index, device_path);

            fdinfo.print(&proc_index, &FdInfoSortType::VRAM, false).unwrap();
            fdinfo.text.set();

            Arc::new(Mutex::new(proc_index))
        };

        Self {
            amdgpu_dev,
            device_path: device_path.clone(),
            instance,
            marketing_name,
            pci_bus,
            sysfs_path,
            cu_number,
            vram_usage,
            sensors,
            check_gfxoff,
            family_name,
            fdinfo,
            arc_proc_index,
            info_text: Default::default(),
        }
    }

    fn info_header() -> TextView {
        let text = format!(concat!(
            "GPU  {name:<name_len$} {pad:9}|{pci:<16}|{vram:^18}|\n",
            "SCLK    MCLK    VDDGFX  Power           | GFX% UMC%  MM% |{gtt:^18}|\n",
            "Temp    {fan:<7} {thr:<THR_LEN$}|"
            ),
            name = "Name",
            name_len = GPU_NAME_LEN,
            pci = " PCI Bus",
            vram = "VRAM Usage",
            gtt = " GTT Usage",
            pad = "",
            fan = "Fan",
            thr = "Throttle_Status",
            THR_LEN = THR_LEN,
        );

        TextView::new(text).no_wrap()
    }

    fn info_text(&mut self) -> TextView {
        TextView::new_with_content(self.info_text.content.clone()).no_wrap()
    }

    fn update_info_text(&mut self) -> Result<(), std::fmt::Error> {
        self.info_text.clear();
        self.vram_usage.update_usage(&self.amdgpu_dev);

        writeln!(
            self.info_text.buf,
            " #{i:<2} [{name:GPU_NAME_LEN$}]({cu:3}CU) | {pci}   |{vu:6}/{vt:6} MiB |",
            i = self.instance,
            name = if GPU_NAME_LEN < self.marketing_name.len() {
                &self.marketing_name[..GPU_NAME_LEN]
            } else {
                &self.marketing_name
            },
            cu = self.cu_number,
            pci = self.pci_bus,
            vu = self.vram_usage.0.vram.heap_usage >> 20,
            vt = self.vram_usage.0.vram.total_heap_size >> 20,
        )?;

        if let Some(sclk) = &self.sensors.sclk {
            write!(self.info_text.buf, "{sclk:4}MHz ")?;
        } else {
            write!(self.info_text.buf, "____MHz ")?;
        }

        if let Some(mclk) = &self.sensors.mclk {
            write!(self.info_text.buf, "{mclk:4}MHz ")?;
        } else {
            write!(self.info_text.buf, "____MHz ")?;
        }

        if let Some(vddgfx) = &self.sensors.vddgfx {
            write!(self.info_text.buf, "{vddgfx:4}mV ")?;
        } else {
            write!(self.info_text.buf, "____mV ")?;
        }

        if let Some(power) = &self.sensors.power {
            if let Some(cap) = &self.sensors.power_cap {
                write!(self.info_text.buf, " {power:>3}/{:>3}W ", cap.current)?;
            } else {
                write!(self.info_text.buf, " {power:>3}/___W ")?;
            }
        } else {
            write!(self.info_text.buf, " ___/___W ")?;
        }

        if self.check_gfxoff {
            match GfxoffStatus::get(self.instance) {
                Ok(GfxoffStatus::InGFXOFF) =>
                    write!(self.info_text.buf, "GFXOFF |")?,
                /* for debug */
                Ok(GfxoffStatus::Unknown(val)) =>
                    write!(self.info_text.buf, "Unknown ({val})|")?,
                _ => write!(self.info_text.buf, "       |")?,
            }
        } else {
            write!(self.info_text.buf, "       |")?;
        }

        let metrics = self.amdgpu_dev.get_gpu_metrics_from_sysfs_path(&self.sysfs_path).ok();
        let activity = if let Some(metrics) = &metrics {
            GpuActivity::from(metrics)
        } else if let FAMILY_NAME::RV = self.family_name {
            // Some Raven/Picasso/Raven2 APU always report gpu_busy_percent as 100.
            // ref: https://gitlab.freedesktop.org/drm/amd/-/issues/1932
            // gpu_metrics is supported from Renoir APU.
            GpuActivity { gfx: None, umc: None, media: None }
        } else {
            GpuActivity::get_from_sysfs(&self.sysfs_path)
        };

        for usage in [activity.gfx, activity.umc, activity.media] {
            if let Some(usage) = usage {
                write!(self.info_text.buf, " {usage:>3}%")?;
            } else {
                write!(self.info_text.buf, " ___%")?;
            }
        }

        writeln!(
            self.info_text.buf,
            " |{gu:>6}/{gt:>6} MiB |",
            gu = self.vram_usage.0.gtt.heap_usage >> 20,
            gt = self.vram_usage.0.gtt.total_heap_size >> 20,
        )?;

        if let Some(temp) = &self.sensors.edge_temp {
            write!(self.info_text.buf, " {:>3}C ", temp.current)?;
        } else {
            write!(self.info_text.buf, " ___C ")?;
        }

        if let Some(fan_rpm) = &self.sensors.fan_rpm {
            write!(self.info_text.buf, "  {fan_rpm:4}RPM ")?;
        } else {
            write!(self.info_text.buf, "  ____RPM ")?;
        }

        if let Some(thr) = metrics.and_then(|m| m.get_throttle_status_info()) {
            let thr = format!("{:?}", thr.get_all_throttler());
            write!(
                self.info_text.buf,
                "{:<THR_LEN$}|",
                if THR_LEN < thr.len() { &thr[..THR_LEN] } else { &thr },
            )?;
        } else {
            write!(
                self.info_text.buf,
                "{:<THR_LEN$}|",
                "N/A",
            )?;
        }

        self.info_text.set();

        Ok(())
    }

    fn fdinfo_panel(&self) -> Panel<TextView> {
        let text = TextView::new_with_content(self.fdinfo.text.content.clone());
        Panel::new(text)
            .title(format!("#{:<2} {}", self.instance, self.marketing_name))
            .title_position(HAlign::Left)
    }

    fn update(&mut self, sample: &Sampling, opt: &ToggleOptions) {
        self.sensors.update(&self.amdgpu_dev);

        if opt.fdinfo {
            let lock = self.arc_proc_index.try_lock();
            if let Ok(vec_info) = lock {
                self.fdinfo.print(&vec_info, &FdInfoSortType::default(), false).unwrap();
                self.fdinfo.stat.interval = sample.to_duration();
            } else {
                self.fdinfo.stat.interval += sample.to_duration();
            }
        } else {
            self.fdinfo.text.clear();
        }

        self.update_info_text().unwrap();
        self.fdinfo.text.set();
    }
}

pub fn run_smi(title: &str, device_path_list: &[DevicePath], interval: u64) {
    let sample = Sampling::low();
    let mut opt = ToggleOptions::default();
    let mut vec_app: Vec<SmiDeviceInfo> = device_path_list.iter().filter_map(|device_path| {
        let amdgpu_dev = device_path.init().ok()?;
        let instance = device_path.get_instance_number()?;

        Some(SmiDeviceInfo::new(amdgpu_dev, device_path, instance))
    }).collect();

    vec_app.sort_by(|a, b| a.instance.cmp(&b.instance));

    let mut siv = cursive::default();
    {
        let mut layout = LinearLayout::vertical().child(TextView::new(title));
        let line = TextContent::new(format!("{:->LINE_LEN$}", ""));
        {
            let mut info = LinearLayout::vertical()
                .child(SmiDeviceInfo::info_header())
                .child(TextView::new_with_content(line.clone()).no_wrap());
            for app in vec_app.iter_mut() {
                app.update(&sample, &opt);
                info.add_child(app.info_text());
                info.add_child(TextView::new_with_content(line.clone()).no_wrap());
            }
            info.remove_child(info.len()-1);
            layout.add_child(Panel::new(info));
        }
        {
            let mut proc = LinearLayout::vertical();
            for app in &vec_app {
                proc.add_child(app.fdinfo_panel());
            }
            let h = HideableView::new(proc).with_name(PROC_TITLE);
            layout.add_child(Panel::new(h).title(PROC_TITLE).title_position(HAlign::Left));
        }
        layout.add_child(TextView::new("\n(p)rocesses (q)uit"));

        siv.add_fullscreen_layer(
            layout
                .scrollable()
                .scroll_y(true)
        );
    }
    {
        let t_index: Vec<(DevicePath, Arc<Mutex<Vec<ProcInfo>>>)> = vec_app.iter().map(|app| {
            (app.device_path.clone(), app.arc_proc_index.clone())
        }).collect();
        let mut buf_index: Vec<ProcInfo> = Vec::new();

        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(interval));

            let all_proc = stat::get_all_processes();

            for (device_path, index) in &t_index {
                stat::update_index_by_all_proc(&mut buf_index, device_path, &all_proc);

                let lock = index.lock();
                if let Ok(mut index) = lock {
                    *index = buf_index.clone();
                }
            }
        });
    }

    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('p', |s| {
        s.call_on_name(PROC_TITLE, |view: &mut HideableView<LinearLayout>| {
            view.set_visible(!view.is_visible());
        });
    });
    siv.set_theme(cursive::theme::Theme::terminal_default());

    let toggle_opt = Arc::new(Mutex::new(opt.clone()));
    siv.set_user_data(toggle_opt.clone());

    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move ||
        loop {
            std::thread::sleep(sample.to_duration()); // 1s

            {
                if let Ok(toggle_opt) = toggle_opt.try_lock() {
                    opt = toggle_opt.clone();
                }
            }

            for app in vec_app.iter_mut() {
                app.update(&sample, &opt);
            }

            cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
        }
    );

    siv.run();
}
