use super::{DeviceHandle, Text, Opt, PcieBw, PANEL_WIDTH};
use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::SENSOR_INFO::SENSOR_TYPE,
};
use std::fmt::{self, Write};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Sensors {
    pub hwmon_path: PathBuf,
    pub cur: PCI::LINK,
    pub max: PCI::LINK,
    pub bus_info: PCI::BUS_INFO,
    pub sclk: Option<u32>,
    pub mclk: Option<u32>,
    pub vddnb: Option<u32>,
    pub vddgfx: Option<u32>,
    pub temp: Option<u32>,
    pub critical_temp: Option<u32>,
    pub power: Option<u32>,
    pub power_cap: Option<u32>,
    pub fan_rpm: Option<u32>,
    pub fan_max_rpm: Option<u32>,
}

impl Sensors {
    pub fn new(amdgpu_dev: &DeviceHandle, pci_bus: &PCI::BUS_INFO) -> Self {
        let hwmon_path = pci_bus.get_hwmon_path().unwrap();
        let cur = pci_bus.get_link_info(PCI::STATUS::Current);
        let max = pci_bus.get_link_info(PCI::STATUS::Max);
        let [sclk, mclk, vddnb, vddgfx, temp, power] = [
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP).ok(),
            amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok(),
        ];
        let critical_temp = Self::parse_hwmon(hwmon_path.join("temp1_crit"))
            .map(|temp| temp.saturating_div(1_000));
        let power_cap = Self::parse_hwmon(hwmon_path.join("power1_cap"))
            .map(|cap| cap.saturating_div(1_000_000));

        let fan_rpm = Self::parse_hwmon(hwmon_path.join("fan1_input"));
        let fan_max_rpm = Self::parse_hwmon(hwmon_path.join("fan1_max"));

        Self {
            hwmon_path,
            cur,
            max,
            bus_info: *pci_bus,
            sclk,
            mclk,
            vddnb,
            vddgfx,
            temp,
            critical_temp,
            power,
            power_cap,
            fan_rpm,
            fan_max_rpm,
        }
    }

    fn parse_hwmon<P: Into<PathBuf>>(path: P) -> Option<u32> {
        std::fs::read_to_string(path.into()).ok()
            .and_then(|file| file.trim_end().parse::<u32>().ok())
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.cur = self.bus_info.get_link_info(PCI::STATUS::Current);
        self.sclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_SCLK).ok();
        self.mclk = amdgpu_dev.sensor_info(SENSOR_TYPE::GFX_MCLK).ok();
        self.vddnb = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDNB).ok();
        self.vddgfx = amdgpu_dev.sensor_info(SENSOR_TYPE::VDDGFX).ok();
        self.temp = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_TEMP).ok();
        self.power = amdgpu_dev.sensor_info(SENSOR_TYPE::GPU_AVG_POWER).ok();
        self.fan_rpm = Self::parse_hwmon(self.hwmon_path.join("fan1_input"));
    }
}

const WIDTH: usize = PANEL_WIDTH / 2;

pub struct SensorsView {
    sensors: Sensors,
    pub text: Text,
}

impl SensorsView {
    pub fn new(amdgpu_dev: &DeviceHandle, pci_bus: &PCI::BUS_INFO) -> Self {
        Self {
            sensors: Sensors::new(amdgpu_dev, pci_bus),
            text: Text::default(),
        }
    }

    pub fn update(&mut self, amdgpu_dev: &DeviceHandle) {
        self.sensors.update(amdgpu_dev);
    }

    pub fn print(&mut self) -> Result<(), fmt::Error> {
        let sensors = &self.sensors;
        const NAME_LEN: usize = 10;
        const VAL_LEN: usize = 5;
        self.text.clear();

        let mut c = 0;

        for (name, val, unit) in [
            ("GFX_SCLK", sensors.sclk, "MHz"),
            ("GFX_MCLK", sensors.mclk, "MHz"),
            ("VDDNB", sensors.vddnb, "mV"),
            ("VDDGFX", sensors.vddgfx, "mV"),
        ] {
            let Some(val) = val else { continue };
            c += 1;
            write!(
                self.text.buf,
                " {:<WIDTH$} ",
                format!("{name:<NAME_LEN$} => {val:>VAL_LEN$} {unit:3}")
            )?;
            if (c % 2) == 0 { writeln!(self.text.buf)? };
        }
        if (c % 2) == 1 { writeln!(self.text.buf)?; }

        if let Some(temp) = sensors.temp {
            let temp = temp.saturating_div(1_000);
            if let Some(crit) = sensors.critical_temp {
                writeln!(self.text.buf, " GPU Temp. => {temp:3} C (Crit. {crit} C)")?;
            } else {
                writeln!(self.text.buf, " GPU Temp. => {temp:3} C")?;
            }
        }
        if let Some(power) = sensors.power {
            if let Some(cap) = sensors.power_cap {
                writeln!(self.text.buf, " GPU Power => {power:3} W (Cap. {cap} W)")?;
            } else {
                writeln!(self.text.buf, " GPU Power => {power:3} W")?;
            }
        }
        if let Some(fan_rpm) = sensors.fan_rpm {
            if let Some(max_rpm) = sensors.fan_max_rpm {
                writeln!(self.text.buf, " Fan => {fan_rpm:4} RPM (Max. {max_rpm} RPM)")?;
            } else {
                writeln!(self.text.buf, " Fan => {fan_rpm:4} RPM")?;
            }
        }

        writeln!(
            self.text.buf,
            " PCI ({pci_bus}) => Gen{cur_gen}x{cur_width:<2} (Max. Gen{max_gen}x{max_width})",
            pci_bus = sensors.bus_info,
            cur_gen = sensors.cur.gen,
            cur_width = sensors.cur.width,
            max_gen = sensors.max.gen,
            max_width = sensors.max.width,
        )?;

        Ok(())
    }

    pub fn print_pcie_bw(&mut self, pcie_bw: &PcieBw) -> Result<(), fmt::Error> {
        let sent = (pcie_bw.sent * pcie_bw.max_payload_size as u64) >> 20; // MiB
        let rec = (pcie_bw.received * pcie_bw.max_payload_size as u64) >> 20; // MiB

        writeln!(
            self.text.buf,
            " PCIe Bandwidth Usage => Sent: {sent:6} MiB/s, Received: {rec:6} MiB/s",
        )?;

        Ok(())
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.sensor ^= true;
        }
    }
}
