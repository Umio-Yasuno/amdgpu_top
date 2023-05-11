use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::align::HAlign;
use cursive::view::Scrollable;
use cursive::views::{LinearLayout, TextView, Panel};

use libamdgpu_top::AMDGPU::DeviceHandle;
use libamdgpu_top::{stat, DevicePath, PCI, Sampling, VramUsage};
use stat::{Sensors, ProcInfo};

use crate::{FdInfoView, Text, stat::FdInfoSortType};

const GPU_NAME_LEN: usize = 25;

pub(crate) struct SmiDeviceInfo {
    pub amdgpu_dev: DeviceHandle,
    pub device_path: DevicePath,
    pub instance: u32,
    pub marketing_name: String,
    pub pci_bus: PCI::BUS_INFO,
    pub vram_usage: VramUsage,
    pub sensors: Sensors,
    pub fdinfo: FdInfoView,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub info_text: Text,
}

impl SmiDeviceInfo {
    pub fn new(amdgpu_dev: DeviceHandle, device_path: &DevicePath, instance: u32) -> Self {
        let marketing_name = {
            let tmp = amdgpu_dev.get_marketing_name().unwrap();

            if GPU_NAME_LEN < tmp.len() {
                tmp[..GPU_NAME_LEN].to_string()
            } else {
                tmp
            }
        };
        let pci_bus = match device_path.pci {
            Some(pci_bus) => pci_bus,
            None => amdgpu_dev.get_pci_bus_info().unwrap(),
        };
        let memory_info = amdgpu_dev.memory_info().unwrap();
        let vram_usage = VramUsage(memory_info);
        let sensors = Sensors::new(&amdgpu_dev, &pci_bus);

        let mut fdinfo = FdInfoView::new(Sampling::default().to_duration());

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
            vram_usage,
            sensors,
            fdinfo,
            arc_proc_index,
            info_text: Default::default(),
        }
    }

    fn info_text(&mut self) -> TextView {
        TextView::new_with_content(self.info_text.content.clone())
    }

    fn update_info_text(&mut self) -> Result<(), std::fmt::Error> {
        self.info_text.clear();
        write!(
            self.info_text.buf,
            " #{i:<2} {name:GPU_NAME_LEN$} | {pci:12} | {usage:5}MiB / {total:5}MiB |",
            i = self.instance,
            name = self.marketing_name,
            pci = self.pci_bus,
            usage = self.vram_usage.0.vram.heap_usage >> 20,
            total = self.vram_usage.0.vram.total_heap_size >> 20,
        )?;

        if let Some(temp) = self.sensors.temp {
            write!(self.info_text.buf, " {temp:>3}C, ")?;
        } else {
            write!(self.info_text.buf, " ___C, ")?;
        }
        if let Some(power) = self.sensors.power {
            if let Some(cap) = self.sensors.power_cap {
                write!(self.info_text.buf, "  {power:>4}W / {cap:>4}W, ")?;
            } else {
                write!(self.info_text.buf, "  {power:>4}W / ____W, ")?;
            }
        } else {
            write!(self.info_text.buf, "  ____W / ____W, ")?;
        }
        if let Some(fan_rpm) = self.sensors.fan_rpm {
            write!(self.info_text.buf, " {fan_rpm:4}RPM ")?;
        } else {
            write!(self.info_text.buf, " ___RPM ")?;
        }

        self.info_text.set();

        Ok(())
    }

    fn update(&mut self, sample: &Sampling) {
        self.vram_usage.update_usage(&self.amdgpu_dev);
        self.sensors.update(&self.amdgpu_dev);

        {
            let lock = self.arc_proc_index.try_lock();
            if let Ok(vec_info) = lock {
                self.fdinfo.print(&vec_info, &FdInfoSortType::default(), false).unwrap();
                self.fdinfo.stat.interval = sample.to_duration();
            } else {
                self.fdinfo.stat.interval += sample.to_duration();
            }
        }

        self.update_info_text().unwrap();
        self.fdinfo.text.set();
    }
}

pub fn run_smi(title: &str, device_path_list: &[DevicePath], interval: u64) {
    let mut vec_app: Vec<SmiDeviceInfo> = device_path_list.iter().filter_map(|device_path| {
        let amdgpu_dev = device_path.init().ok()?;
        let instance = device_path.get_instance_number()?;

        Some(SmiDeviceInfo::new(amdgpu_dev, device_path, instance))
    }).collect();

    vec_app.sort_by(|a, b| a.instance.cmp(&b.instance));

    let mut siv = cursive::default();
    {
        let mut layout = LinearLayout::vertical()
            .child(TextView::new(title));

        let mut info = LinearLayout::vertical()
            .child(TextView::new(format!(
                "GPU  {name:<GPU_NAME_LEN$} | {pci:<12} | {vram:<19} | Temp.  Pwr Usage/Cap    Fan",
                name = "Name",
                pci = "PCI Bus",
                vram = "VRAM Usage",
            )));
        for app in vec_app.iter_mut() {
            app.update_info_text().unwrap();
            info.add_child(app.info_text());
        }
        layout.add_child(Panel::new(info));

        for app in &vec_app {
            let text = TextView::new_with_content(app.fdinfo.text.content.clone());
            layout.add_child(
                Panel::new(text)
                    .title(format!("#{} Processes", app.instance))
                    .title_position(HAlign::Left)
            );
        }

        layout.add_child(TextView::new("\n(q)uit"));

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
    siv.set_theme(cursive::theme::Theme::terminal_default());

    let cb_sink = siv.cb_sink().clone();
    let sample = Sampling::low();

    std::thread::spawn(move || loop {
        std::thread::sleep(sample.to_duration()); // 1s

        for app in vec_app.iter_mut() {
            app.update(&sample);
        }

        cb_sink.send(Box::new(cursive::Cursive::noop)).unwrap();
    });

    siv.run();
}
