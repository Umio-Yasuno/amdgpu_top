use std::time::Duration;
use libamdgpu_top::{
    AMDGPU::{
        VIDEO_CAPS::CODEC,
        FW_VERSION::{FW_TYPE, FwVer},
        DpmClockType,
        DpmClockRange,
        GPU_INFO,
        GpuMetrics,
        MetricsInfo,
    },
    app::AppAmdgpuTop,
    AppDeviceInfo,
    // DeviceHandle,
    DevicePath,
    stat::Sensors,
};
use crate::{OptDumpMode, drm_info};

pub fn dump_gpu_metrics(title: &str, device_path_list: &[DevicePath]) {
    println!("{title}");

    for (i, device_path) in device_path_list.iter().enumerate() {
        println!("\n--------\n#{i}");

        if let [Some(device_id), Some(rev_id)] = [device_path.device_id, device_path.revision_id] {
            println!("{} ({}, {device_id:#0X}:{rev_id:#0X})", device_path.device_name, device_path.pci);
        } else {
            println!("{} ({})", device_path.device_name, device_path.pci);
        }

        if let Ok(m) = GpuMetrics::get_from_sysfs_path(&device_path.sysfs_path) {
            println!("\nGPU Metrics: {m:#?}");
        } else {
            println!("\nGPU Metrics: Not Supported");
        }
    }
}

pub fn dump_all(title: &str, device_path_list: &[DevicePath], opt_dump_mode: OptDumpMode) {
    println!("{title}");

    if let Some(ver) = libamdgpu_top::get_rocm_version() {
        println!("ROCm Version: {ver}");
    }

    for (i, device_path) in device_path_list.iter().enumerate() {
        println!("\n--------\n#{i} {device_path:#X?}");
        dump(device_path, opt_dump_mode);
    }
}

pub fn dump(device_path: &DevicePath, opt_dump_mode: OptDumpMode) {
    let amdgpu_dev = device_path.init().unwrap();
    let drm = amdgpu_dev.get_drm_version_struct();
    let Some(app) = AppAmdgpuTop::new(amdgpu_dev, device_path.clone(), &Default::default())
        .map(|mut app| {
            app.update(Duration::ZERO);
            app
        }) else {
            return;
        };
    let sensors = &app.stat.sensors;
    let info = &app.device_info;

    if let Ok(drm) = drm {
        println!("drm version: {}.{}.{}", drm.version_major, drm.version_minor, drm.version_patchlevel);
    }

    info.device_info();

    if let Some(ver) = info
        .gfx_target_version.clone()
        .or_else(|| device_path.get_gfx_target_version_from_kfd().map(|v| v.to_string()))
    {
        println!("gfx_target_version       : {ver}");
    }

    info.gfx_info();
    info.memory_info();

    if let Some(sensors) = sensors {
        sensors_info(sensors);
    }

    {
        let profiles: Vec<String> = info.power_profiles.iter().map(|p| p.to_string()).collect();
        if !profiles.is_empty() {
            println!("\nSupported Power Profiles: {profiles:?}");
        }
    }

    info.cache_info();
    info.hw_ip_info();

    if !info.ip_die_entries.is_empty() {
        info.ip_discovery_table();
    }

    info.fw_info();
    info.codec_info();
    info.vbios_info();

    let pp_feature_mask = libamdgpu_top::PpFeatureMask::get_all_enabled_feature();

    if !pp_feature_mask.is_empty() {
        println!("\npp_feature_mask: {pp_feature_mask:#?}");
    }

    if let OptDumpMode::GpuMetrics = opt_dump_mode {
        if let Some(m) = app.stat.metrics {
            println!("\nGPU Metrics: {m:#?}");
        } else {
            println!("\nGPU Metrics: Not Supported");
        }
    } else if let Some(h) = app.stat.metrics.as_ref().and_then(|m| m.get_header()) {
        println!("\nGPU Metrics Version: v{}.{}", h.format_revision, h.content_revision);
    }

    if let OptDumpMode::DrmInfo = opt_dump_mode {
        drm_info::dump_drm_info(device_path);
    }
}

fn sensors_info(sensors: &Sensors) {
    println!();

    for temp in [&sensors.edge_temp, &sensors.junction_temp, &sensors.memory_temp] {
        let Some(temp) = temp else { continue };
        let label = format!("{} Temp.", temp.type_);
        print!("{label:<15} : {:>3} C (Current)", temp.current);
        if let Some(crit) = &temp.critical {
            print!(", {crit:>3} C (Critical)");
        }
        if let Some(e) = &temp.emergency {
            print!(", {e:>3} C (Emergency)");
        }
        println!();
    }

    if let Some(ref tctl) = sensors.tctl {
        println!("CPU Tctl        : {:3} C", tctl / 1000);
    }

    println!();
    for power in [&sensors.average_power, &sensors.input_power] {
        let Some(power) = power else { continue };
        println!("Power ({:<7})     : {:3} W", power.type_.to_string(), power.value);
    }
    if let Some(cap) = &sensors.power_cap {
        println!("Power Cap.          : {:3} W ({}-{} W)", cap.current, cap.min, cap.max);
        println!("Power Cap. (Default): {:3} W", cap.default);
    }
    if let Some(fan_max_rpm) = &sensors.fan_max_rpm {
        println!("Fan RPM (Max)       : {fan_max_rpm} RPM");
    }
    if let Some(fan_rpm) = &sensors.fan_rpm {
        println!("Fan RPM (Current)   : {fan_rpm} RPM");
    }

    const PCIE_LABEL: &str = "PCIe Link Speed";
    const PCIE_LEN: usize = 14;

    if let [Some(min), Some(max)] = [&sensors.min_dpm_link, &sensors.max_dpm_link] {
        println!(
            "{PCIE_LABEL} {:PCIE_LEN$}: Gen{}x{:<2} - Gen{}x{:<2}",
            "(DPM, Min-Max)",
            min.r#gen,
            min.width,
            max.r#gen,
            max.width,
        );
    } else if let Some(max) = &sensors.max_dpm_link {
        println!("{PCIE_LABEL} {:PCIE_LEN$}: Gen{}x{:<2}", "(DPM, Max)", max.r#gen, max.width);
    }

    for (link, label) in [
        (&sensors.max_gpu_link, "(GPU, Max)"),
        (&sensors.max_system_link, "(System, Max)"),
    ] {
        let Some(link) = link else { continue };
        println!("{PCIE_LABEL} {label:PCIE_LEN$}: Gen{}x{:<2}", link.r#gen, link.width);
    }

    println!();

    if let Some(f) = &sensors.fclk_dpm {
        println!("  FCLK (DPM): {:>4}MHz ({:>4}-{:>4}MHz)", f.current_mhz, f.min_mhz, f.max_mhz);
    }
    if let Some(s) = DpmClockRange::from_sysfs(DpmClockType::SOCCLK, &sensors.sysfs_path) {
        println!("SOCCLK (DPM): {:>4}MHz ({:>4}-{:>4}MHz)", s.current_mhz, s.min_mhz, s.max_mhz);
    }

    if !sensors.all_cpu_core_freq_info.is_empty() {
        println!("\nCPU Core freq: <MIN>-<CUR>-<MAX>")
    }

    for freq_info in &sensors.all_cpu_core_freq_info {
        println!(
            "    Core{:<2}:     {:4}--{:4}--{:4} MHz",
            freq_info.core_id,
            freq_info.min,
            freq_info.cur,
            freq_info.max,
        );
    }
}

trait DumpInfo {
    fn device_info(&self);
    fn gfx_info(&self);
    fn memory_info(&self);
    fn cache_info(&self);
    fn vbios_info(&self);
    fn codec_info(&self);
    fn hw_ip_info(&self);
    fn ip_discovery_table(&self);
    fn fw_info(&self);
}

impl DumpInfo for AppDeviceInfo {
    fn device_info(&self) {
        println!();
        println!("Device Name              : [{}]", self.marketing_name);
        println!("PCI (domain:bus:dev.func): {}", self.pci_bus);
        println!(
            "DeviceID.RevID           : {:#0X}.{:#0X}",
            self.ext_info.device_id(),
            self.ext_info.pci_rev_id()
        );
    }

    fn gfx_info(&self) {
        let asic = self.ext_info.get_asic_name();

        println!();
        println!("GPU Type  : {}", if self.ext_info.is_apu() { "APU" } else { "dGPU" });
        println!("Family    : {}", self.ext_info.get_family_name());
        println!("ASIC Name : {asic}");
        println!("Chip Class: {}", self.ext_info.get_chip_class());

        let max_good_cu_per_sa = self.ext_info.get_max_good_cu_per_sa();
        let min_good_cu_per_sa = self.ext_info.get_min_good_cu_per_sa();

        println!();
        println!("Shader Engine (SE)         : {:3}", self.ext_info.max_se());
        println!("Shader Array (SA/SH) per SE: {:3}", self.ext_info.max_sa_per_se());
        if max_good_cu_per_sa != min_good_cu_per_sa {
            println!("CU per SA[0]               : {:3}", max_good_cu_per_sa);
            println!("CU per SA[1]               : {:3}", min_good_cu_per_sa);
        } else {
            println!("CU per SA                  : {:3}", max_good_cu_per_sa);
        }
        println!("Total Compute Unit         : {:3}", self.ext_info.cu_active_number());

        let rb_pipes = self.ext_info.rb_pipes();
        let rop_count = self.ext_info.calc_rop_count();

        if asic.rbplus_allowed() {
            println!("RenderBackendPlus (RB+)    : {rb_pipes:3} ({rop_count} ROPs)");
        } else {
            println!("RenderBackend (RB)         : {rb_pipes:3} ({rop_count} ROPs)");
        }

        println!("Peak Pixel Fill-Rate       : {:3} GP/s", rop_count * self.max_gpu_clk / 1000);

        println!();
        println!("GPU Clock: {}-{} MHz", self.min_gpu_clk, self.max_gpu_clk);
        println!("Peak FP32: {} GFLOPS", self.ext_info.peak_gflops());
    }

    fn memory_info(&self) {
        let resizable_bar = if self.resizable_bar {
            "Enabled"
        } else {
            "Disabled"
        };

        println!();
        println!("VRAM Type     : {}", self.ext_info.get_vram_type());
        println!("VRAM Bit Width: {}-bit", self.ext_info.vram_bit_width);

        if let Some(mem_vendor) = &self.memory_vendor {
            println!("VRAM Vendor   : {mem_vendor}");
        }

        println!("Memory Clock  : {}-{} MHz", self.min_mem_clk, self.max_mem_clk);
        println!("Peak Memory BW: {} GB/s", self.ext_info.peak_memory_bw_gb());
        println!("ResizableBAR  : {resizable_bar}");
        println!();

        for (label, mem) in [
            ("VRAM", &self.memory_info.vram),
            ("CPU-Visible VRAM", &self.memory_info.cpu_accessible_vram),
            ("GTT", &self.memory_info.gtt),
        ] {
            println!(
                "{label:<18}: usage {:5} MiB, total {:5} MiB (usable {:5} MiB)",
                mem.heap_usage >> 20,
                mem.total_heap_size >> 20,
                mem.usable_heap_size >> 20,
            );
        }
    }

    fn cache_info(&self) {
        println!();
        println!("L1 Cache (per CU)    : {:4} KiB", self.l1_cache_size_kib_per_cu);
        if 0 < self.gl1_cache_size_kib_per_sa {
            println!("GL1 Cache (per SA/SH): {:4} KiB", self.gl1_cache_size_kib_per_sa);
        }
        println!(
            "L2 Cache             : {:4} KiB ({} Banks)",
            self.total_l2_cache_size_kib,
            self.actual_num_tcc_blocks,
        );
        if 0 < self.total_l3_cache_size_mib {
            println!("L3 Cache             : {:4} MiB", self.total_l3_cache_size_mib);
        }
    }

    fn vbios_info(&self) {
        let Some(vbios) = &self.vbios else { return };

        println!("\nVBIOS info:");
        println!("    name   : [{}]", vbios.name);
        println!("    pn     : [{}]", vbios.pn);
        println!("    ver_str: [{}]", vbios.ver);
        println!("    date   : [{}]", vbios.date);
    }

    fn codec_info(&self) {
        let [Some(decode), Some(encode)] = [&self.decode, &self.encode] else { return };
        println!("\nVideo caps (WIDTHxHEIGHT):");

        for (codec, dec_cap, enc_cap) in [
            (CODEC::MPEG2, decode.mpeg2, encode.mpeg2),
            (CODEC::MPEG4, decode.mpeg4, encode.mpeg4),
            (CODEC::VC1, decode.vc1, encode.vc1),
            (CODEC::MPEG4_AVC, decode.mpeg4_avc, encode.mpeg4_avc),
            (CODEC::HEVC, decode.hevc, encode.hevc),
            (CODEC::JPEG, decode.jpeg, encode.jpeg),
            (CODEC::VP9, decode.vp9, encode.vp9),
            (CODEC::AV1, decode.av1, encode.av1),
        ] {
            let codec = codec.to_string();
            let [dec, enc] = [dec_cap, enc_cap].map(|cap| {
                if let Some(cap) = cap {
                    format!("{}x{}", cap.max_width, cap.max_height)
                } else {
                    "N/A".to_string()
                }
            });
            println!("    {codec:10}: {dec:>12} (Decode), {enc:>12} (Encode)");
        }
    }

    fn hw_ip_info(&self) {
        println!("\nHardware IP info:");

        for hw_ip_info in &self.hw_ip_info_list {
            println!(
                "    {ip_type:8} count: {ip_count}, ver: {major:2}.{minor}, queues: {queues}",
                ip_type = hw_ip_info.ip_type.to_string(),
                ip_count = hw_ip_info.count,
                major = hw_ip_info.info.hw_ip_version_major,
                minor = hw_ip_info.info.hw_ip_version_minor,
                queues = hw_ip_info.info.num_queues(),
            );
        }
    }

    fn ip_discovery_table(&self) {
        println!("\nIP Discovery table:");
        for die in &self.ip_die_entries {
            println!("    die_id: {:>2}", die.die_id);

            for ip_hw in &die.ip_hw_ids {
                let hw_id = ip_hw.hw_id.to_string();
                let Some(inst_info) = ip_hw.instances.first() else { continue };
                println!(
                    "        {hw_id:<10} num: {}, ver: {:>3}.{}.{}",
                    ip_hw.instances.len(),
                    inst_info.major,
                    inst_info.minor,
                    inst_info.revision,
                );
            }
        }
    }

    fn fw_info(&self) {
        fn fw_ver_dump(fw_info: &FwVer) {
            let (ver, ftr) = (fw_info.version, fw_info.feature);

            if ver == 0 { return; }

            let is_mec2 = (FW_TYPE::GFX_MEC, 0, 1) == (fw_info.fw_type, fw_info.ip_instance, fw_info.index);

            let fw_type = if is_mec2 {
                "GFX_MEC2".to_string()
            } else {
                fw_info.fw_type.to_string()
            };

            println!(
                "    {fw_type:<8} feature: {ftr:>3}, ver: {ver:>#10X}",
            );
        }

        println!("\nFirmware info:");

        for fw_ver in &self.fw_versions {
            fw_ver_dump(fw_ver);
        }
    }
}

pub fn decode_gpu_metrics(path: &str) -> GpuMetrics {
    let mut buf = Vec::with_capacity(128);

    GpuMetrics::read_file_with_buffer(&mut buf, path).unwrap()
}
