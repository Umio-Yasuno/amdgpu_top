use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use cursive::align::HAlign;
use cursive::view::Scrollable;
use cursive::views::{LinearLayout, TextView, Panel};

use libamdgpu_top::AMDGPU::{DeviceHandle, GPU_INFO};
use libamdgpu_top::{stat, DevicePath, PCI, Sampling, VramUsage};
use stat::{Sensors, ProcInfo};

use crate::{FdInfoView, Text, ToggleOptions, stat::FdInfoSortType};

const GPU_NAME_LEN: usize = 25;

pub(crate) struct SmiDeviceInfo {
    pub amdgpu_dev: DeviceHandle,
    pub device_path: DevicePath,
    pub instance: u32,
    pub marketing_name: String,
    pub pci_bus: PCI::BUS_INFO,
    pub cu_number: u32,
    pub vram_usage: VramUsage,
    pub vram_text: Text,
    pub sensors: Sensors,
    pub fdinfo: FdInfoView,
    pub arc_proc_index: Arc<Mutex<Vec<ProcInfo>>>,
    pub info_text: Text,
}

impl SmiDeviceInfo {
    pub fn new(amdgpu_dev: DeviceHandle, device_path: &DevicePath, instance: u32) -> Self {
        let marketing_name = amdgpu_dev.get_marketing_name().unwrap();
        let pci_bus = match device_path.pci {
            Some(pci_bus) => pci_bus,
            None => amdgpu_dev.get_pci_bus_info().unwrap(),
        };
        let ext_info = amdgpu_dev.device_info().unwrap();
        let cu_number = ext_info.cu_active_number();
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
            cu_number,
            vram_usage,
            vram_text: Default::default(),
            sensors,
            fdinfo,
            arc_proc_index,
            info_text: Default::default(),
        }
    }

    fn info_header() -> TextView {
        let text = format!(
            "GPU  {name:<GPU_NAME_LEN$} {pad:7} | {pci:<12} | VRAM% | GTT% | Temp.  Pwr Avg/Cap   Fan",
            name = "Name",
            pci = "PCI Bus",
            pad = "",
        );

        TextView::new(text)
    }

    fn info_text(&mut self) -> TextView {
        TextView::new_with_content(self.info_text.content.clone())
    }

    fn update_info_text(&mut self) -> Result<(), std::fmt::Error> {
        self.info_text.clear();
        let [vram, gtt] = [&self.vram_usage.0.vram, &self.vram_usage.0.gtt].map(|v| {
            (v.heap_usage * 100).checked_div(v.total_heap_size).unwrap_or(0)
        });

        write!(
            self.info_text.buf,
            " #{i:<2} {name:GPU_NAME_LEN$} ({cu:3}CU) | {pci:12} |  {vram:3}% | {gtt:3}% |",
            i = self.instance,
            name = self.marketing_name,
            cu = self.cu_number,
            pci = self.pci_bus,
            vram = vram,
            gtt = gtt,
        )?;

        if let Some(temp) = &self.sensors.edge_temp {
            write!(self.info_text.buf, " {:>3}C, ", temp.current)?;
        } else {
            write!(self.info_text.buf, " ___C, ")?;
        }
        if let Some(power) = self.sensors.power {
            if let Some(ref cap) = self.sensors.power_cap {
                write!(self.info_text.buf, " {power:>3}W / {:>3}W, ", cap.current)?;
            } else {
                write!(self.info_text.buf, " {power:>3}W / ___W, ")?;
            }
        } else {
            write!(self.info_text.buf, "  ____W / ____W, ")?;
        }
        if let Some(fan_rpm) = self.sensors.fan_rpm {
            write!(self.info_text.buf, " {fan_rpm:4}RPM ")?;
        } else {
            write!(self.info_text.buf, " ____RPM ")?;
        }

        self.info_text.set();

        Ok(())
    }

    fn vram_view(&self) -> TextView {
        TextView::new_with_content(self.vram_text.content.clone())
    }

    fn fdinfo_panel(&self) -> Panel<TextView> {
        let text = TextView::new_with_content(self.fdinfo.text.content.clone());
        Panel::new(text)
            .title("Processes")
            .title_position(HAlign::Left)
    }

    fn update_vram(&mut self) -> Result<(), std::fmt::Error> {
        self.vram_text.clear();
        self.vram_usage.update_usage(&self.amdgpu_dev);
        write!(
            self.vram_text.buf,
            " | VRAM: {vu:5} / {vt:5} MiB | GTT: {gu:5} / {gt:5} MiB |",
            vu = self.vram_usage.0.vram.heap_usage >> 20,
            vt = self.vram_usage.0.vram.total_heap_size >> 20,
            gu = self.vram_usage.0.gtt.heap_usage >> 20,
            gt = self.vram_usage.0.gtt.total_heap_size >> 20,
        )?;

        self.vram_text.set();

        Ok(())
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

        self.update_vram().unwrap();
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
        let mut layout = LinearLayout::vertical()
            .child(TextView::new(title));

        let mut info = LinearLayout::vertical()
            .child(SmiDeviceInfo::info_header());
        for app in vec_app.iter_mut() {
            //app.update_info_text().unwrap();
            app.update(&sample, &opt);
            info.add_child(app.info_text());
        }
        layout.add_child(Panel::new(info));

        for app in &vec_app {
            let mut sub = LinearLayout::vertical();
            sub.add_child(app.vram_view());
            sub.add_child(app.fdinfo_panel());
            layout.add_child(
                Panel::new(sub)
                    .title(format!("#{i:<2} {name}", i = app.instance, name = app.marketing_name))
                    .title_position(HAlign::Left)
            );
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
    siv.add_global_callback('p', FdInfoView::cb);
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
