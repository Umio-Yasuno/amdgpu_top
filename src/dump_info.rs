use libamdgpu_top::{
    AMDGPU::{
        VIDEO_CAPS::CODEC,
        FW_VERSION::FW_TYPE,
        DeviceHandle,
        GPU_INFO,
    },
    AppDeviceInfo,
    stat::Sensors,
};

pub fn dump(amdgpu_dev: &DeviceHandle) {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let sensors = Sensors::new(amdgpu_dev, &pci_bus, &ext_info);

    let info = AppDeviceInfo::new(amdgpu_dev, &ext_info, &memory_info, &sensors);

    println!("--- AMDGPU info dump ---");
    if let Ok(drm) = amdgpu_dev.get_drm_version_struct() {
        println!("drm version: {}.{}.{}", drm.version_major, drm.version_minor, drm.version_patchlevel);
    }

    info.device_info();
    info.gfx_info();
    info.memory_info();
    sensors_info(&sensors);
    info.cache_info();
    if !info.ip_die_entries.is_empty() {
        info.ip_discovery_table();
    }
    fw_info(amdgpu_dev);
    info.codec_info();
    info.vbios_info();
    if let Ok(metrics) = amdgpu_dev.get_gpu_metrics() {
        println!("\nGPU Metrics {metrics:#?}");
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
    println!();
    if let Some(power) = &sensors.power {
        println!("Power Avg.          : {power:3} W");
    }
    if let Some(cap) = &sensors.power_cap {
        println!("Power Cap.          : {:3} W ({}-{} W)", cap.current, cap.min, cap.max);
        println!("Power Cap. (Default): {:3} W", cap.default);
    }
    if let Some(fan_max_rpm) = &sensors.fan_max_rpm {
        println!("Fan RPM (Max)       : {fan_max_rpm} RPM");
    }

    const PCIE_LABEL: &str = "PCIe Link Speed";
    const PCIE_LEN: usize = 14;

    if let [Some(min), Some(max)] = [&sensors.min_dpm_link, &sensors.max_dpm_link] {
        println!(
            "{PCIE_LABEL} {:PCIE_LEN$}: Gen{}x{:<2} - Gen{}x{:<2}",
            "(DPM, Min-Max)",
            min.gen,
            min.width,
            max.gen,
            max.width,
        );
    } else if let Some(max) = &sensors.max_dpm_link {
        println!("{PCIE_LABEL} {:PCIE_LEN$}: Gen{}x{:<2}", "(DPM, Max)", max.gen, max.width);
    }

    for (link, label) in [
        (&sensors.min_dpm_link, "(DPM, Min)"),
        (&sensors.max_dpm_link, "(DPM, Max)"),
        (&sensors.max_gpu_link, "(GPU, Max)"),
        (&sensors.max_system_link, "(System, Max)"),
    ] {
        let Some(link) = link else { continue };
        println!("{PCIE_LABEL} {label:PCIE_LEN$}: Gen{}x{:<2}", link.gen, link.width);
    }
}

fn fw_info(amdgpu_dev: &DeviceHandle) {
    const FW_LIST: &[FW_TYPE] = &[
        FW_TYPE::VCE,
        FW_TYPE::UVD,
        FW_TYPE::GMC,
        FW_TYPE::GFX_ME,
        FW_TYPE::GFX_PFP,
        FW_TYPE::GFX_CE,
        FW_TYPE::GFX_RLC,
        FW_TYPE::GFX_MEC,
        FW_TYPE::SMC,
        FW_TYPE::SDMA,
        FW_TYPE::SOS,
        FW_TYPE::ASD,
        FW_TYPE::VCN,
        FW_TYPE::GFX_RLC_RESTORE_LIST_CNTL,
        FW_TYPE::GFX_RLC_RESTORE_LIST_GPM_MEM,
        FW_TYPE::GFX_RLC_RESTORE_LIST_SRM_MEM,
        FW_TYPE::DMCU,
        FW_TYPE::TA,
        FW_TYPE::DMCUB,
        FW_TYPE::TOC,
    ];

    println!("\nFirmware info:");

    for fw_type in FW_LIST {
        let fw_info = match amdgpu_dev.query_firmware_version(*fw_type, 0, 0) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let (ver, ftr) = (fw_info.version, fw_info.feature);

        if ver == 0 { continue }

        println!(
            "    {fw_type:<8} feature: {ftr:>3}, ver: {ver:>#10X}",
            fw_type = fw_type.to_string(),
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
    fn ip_discovery_table(&self);
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
        println!("Chip class: {}", self.ext_info.get_chip_class());

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
}
