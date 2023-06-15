use libamdgpu_top::{
    AMDGPU::{
        VIDEO_CAPS::{CODEC, VideoCapsInfo},
        HW_IP::HwIpInfo,
        FW_VERSION::FW_TYPE,
        DeviceHandle,
        GPU_INFO,
        drm_amdgpu_info_device,
        VBIOS::VbiosInfo,
    },
    AppDeviceInfo,
    stat::Sensors,
};

pub fn dump(amdgpu_dev: &DeviceHandle) {
    let ext_info = amdgpu_dev.device_info().unwrap();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();
    let sensors = Sensors::new(amdgpu_dev, &pci_bus);
    let asic = ext_info.get_asic_name();

    let info = AppDeviceInfo::new(amdgpu_dev, &ext_info, &memory_info, &sensors);

    let (min_gpu_clk, max_gpu_clk) = amdgpu_dev.get_min_max_gpu_clock()
        .unwrap_or_else(|| (0, (ext_info.max_engine_clock() / 1000) as u32));
    let (min_mem_clk, max_mem_clk) = amdgpu_dev.get_min_max_memory_clock()
        .unwrap_or_else(|| (0, (ext_info.max_memory_clock() / 1000) as u32));

    println!("--- AMDGPU info dump ---");
    if let Ok(drm) = amdgpu_dev.get_drm_version_struct() {
        println!("drm version: {}.{}.{}", drm.version_major, drm.version_minor, drm.version_patchlevel);
    }
    println!();

    println!("Device Name              : [{}]", info.marketing_name);
    println!("PCI (domain:bus:dev.func): {pci_bus}");
    println!(
        "DeviceID.RevID           : {:#0X}.{:#0X}",
        ext_info.device_id(),
        ext_info.pci_rev_id()
    );

    println!();
    println!("GPU Type  : {}", if ext_info.is_apu() { "APU" } else { "dGPU" });
    println!("Family    : {}", ext_info.get_family_name());
    println!("ASIC Name : {asic}");
    println!("Chip class: {}", ext_info.get_chip_class());

    let max_good_cu_per_sa = ext_info.get_max_good_cu_per_sa();
    let min_good_cu_per_sa = ext_info.get_min_good_cu_per_sa();

    println!();
    println!("Shader Engine (SE)         : {:3}", ext_info.max_se());
    println!("Shader Array (SA/SH) per SE: {:3}", ext_info.max_sa_per_se());
    if max_good_cu_per_sa != min_good_cu_per_sa {
        println!("CU per SA[0]               : {:3}", max_good_cu_per_sa);
        println!("CU per SA[1]               : {:3}", min_good_cu_per_sa);
    } else {
        println!("CU per SA                  : {:3}", max_good_cu_per_sa);
    }
    println!("Total Compute Unit         : {:3}", ext_info.cu_active_number());

    let rb_pipes = ext_info.rb_pipes();
    let rop_count = ext_info.calc_rop_count();

    if asic.rbplus_allowed() {
        println!("RenderBackendPlus (RB+)    : {rb_pipes:3} ({rop_count} ROPs)");
    } else {
        println!("RenderBackend (RB)         : {rb_pipes:3} ({rop_count} ROPs)");
    }

    println!("Peak Pixel Fill-Rate       : {:3} GP/s", rop_count * max_gpu_clk / 1000);

    println!();
    println!("GPU Clock: {min_gpu_clk}-{max_gpu_clk} MHz");
    println!("Peak FP32: {} GFLOPS", ext_info.peak_gflops());

    let resizable_bar = if info.resizable_bar {
        "Enabled"
    } else {
        "Disabled"
    };

    println!();
    println!("VRAM Type     : {}", ext_info.get_vram_type());
    println!("VRAM Bit Width: {}-bit", ext_info.vram_bit_width);
    println!("Memory Clock  : {min_mem_clk}-{max_mem_clk} MHz");
    println!("Peak Memory BW: {} GB/s", ext_info.peak_memory_bw_gb());
    println!("ResizableBAR  : {resizable_bar}");
    println!();

    for (label, mem) in [
        ("VRAM", &memory_info.vram),
        ("CPU-Visible VRAM", &memory_info.cpu_accessible_vram),
        ("GTT", &memory_info.gtt),
    ] {
        println!(
            "{label:<18}: usage {:5} MiB, total {:5} MiB (usable {:5} MiB)",
            mem.heap_usage >> 20,
            mem.total_heap_size >> 20,
            mem.usable_heap_size >> 20,
        );
    }

    sensors_info(&sensors);
    cache_info(&ext_info);
    hw_ip_info(&info.hw_ip_info);
    fw_info(amdgpu_dev);
    if let [Some(dec), Some(enc)] = [&info.decode, &info.encode] {
        codec_info(dec, enc);
    }
    if let Some(vbios) = &info.vbios {
        vbios_info(vbios);
    }
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
    if sensors.has_pcie_dpm {
        println!("PCIe Link Speed     : Gen{}x{} (Max)", sensors.max.gen, sensors.max.width);
    }
}

fn cache_info(ext_info: &drm_amdgpu_info_device) {
    let gl1_cache_size = ext_info.get_gl1_cache_size() >> 10;
    let l3_cache_size = ext_info.calc_l3_cache_size_mb();

    println!();
    println!("L1 Cache (per CU)    : {:4} KiB", ext_info.get_l1_cache_size() >> 10);
    if 0 < gl1_cache_size {
        println!("GL1 Cache (per SA/SH): {gl1_cache_size:4} KiB");
    }
    println!(
        "L2 Cache             : {:4} KiB ({} Banks)",
        ext_info.calc_l2_cache_size() >> 10,
        ext_info.num_tcc_blocks
    );
    if 0 < l3_cache_size {
        println!("L3 Cache             : {l3_cache_size:4} MiB");
    }
}

fn hw_ip_info(hw_ip_list: &[HwIpInfo]) {
    println!("\nHardware IP info:");

    for hw_ip in hw_ip_list {
        println!(
            "    {ip_type:8} count: {ip_count}, ver: {major:2}.{minor}, queues: {queues}",
            ip_type = hw_ip.ip_type.to_string(),
            ip_count = hw_ip.count,
            major = hw_ip.info.hw_ip_version_major,
            minor = hw_ip.info.hw_ip_version_minor,
            queues = hw_ip.info.num_queues(),
        );
    }
}

fn fw_info(amdgpu_dev: &DeviceHandle) {
    let fw_list = [
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

    for fw_type in &fw_list {
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

fn vbios_info(vbios: &VbiosInfo) {
    println!("\nVBIOS info:");
    println!("    name   : [{}]", vbios.name);
    println!("    pn     : [{}]", vbios.pn);
    println!("    ver_str: [{}]", vbios.ver);
    println!("    date   : [{}]", vbios.date);
}

fn codec_info(decode: &VideoCapsInfo, encode: &VideoCapsInfo) {
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
