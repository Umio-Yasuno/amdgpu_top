use libdrm_amdgpu_sys::{
    PCI,
    AMDGPU::{GPU_INFO, DeviceHandle, drm_amdgpu_info_device},
    AMDGPU::VIDEO_CAPS::{CAP_TYPE, CODEC},
    AMDGPU::HW_IP::HW_IP_TYPE,
    AMDGPU::FW_VERSION::FW_TYPE,
};
use crate::misc;

pub fn dump(amdgpu_dev: &DeviceHandle, major: u32, minor: u32) {
    let ext_info = amdgpu_dev.device_info().unwrap();
    // let chip_class = ext_info.get_chip_class();
    let memory_info = amdgpu_dev.memory_info().unwrap();
    let pci_bus = amdgpu_dev.get_pci_bus_info().unwrap();

    let (min_gpu_clk, min_memory_clk) = misc::get_min_clk(&amdgpu_dev, &pci_bus);
    let gpu_type = if ext_info.is_apu() {
        "APU"
    } else {
        "dGPU"
    };

    println!("--- AMDGPU info dump ---");
    println!("drm version: {major}.{minor}");
    println!();

    if let Ok(mark_name) = amdgpu_dev.get_marketing_name() {
        println!("Marketing Name: [{mark_name}]");
    }

    println!(
        "DeviceID.RevID: {:#0X}.{:#0X}",
        ext_info.device_id(),
        ext_info.pci_rev_id()
    );

    println!();
    println!("GPU Type  : {gpu_type}");
    println!("Family    : {}", ext_info.get_family_name());
    println!("ASIC Name : {}", ext_info.get_asic_name());
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

    println!("RenderBackend (RB)         : {:3} ({} ROP)", ext_info.rb_pipes(), ext_info.calc_rop_count());

    println!();
    println!("GPU Clock: {min_gpu_clk}-{} MHz", ext_info.max_engine_clock() / 1000);
    println!("Peak FP32: {} GFLOPS", ext_info.peak_gflops());

    /* ref: https://gitlab.freedesktop.org/mesa/mesa/blob/main/src/amd/common/ac_gpu_info.c */
    let resizable_bar = if (memory_info.vram.total_heap_size * 9 / 10) <= memory_info.cpu_accessible_vram.total_heap_size {
        "Enabled"
    } else {
        "Disabled"
    };

    println!();
    println!("VRAM Type     : {}", ext_info.get_vram_type());
    println!("VRAM Bit Width: {}-bit", ext_info.vram_bit_width);
    println!("VRAM size     : {} MiB", memory_info.vram.total_heap_size >> 20);
    println!("Memory Clock  : {min_memory_clk}-{} MHz", ext_info.max_memory_clock() / 1000);
    println!("Peak Memory BW: {} GB/s", ext_info.peak_memory_bw_gb());
    println!("ResizableBAR  : {resizable_bar}");

    cache_info(&ext_info);
    pci_info(&pci_bus);
    hw_ip_info(&amdgpu_dev);
    fw_info(&amdgpu_dev);
    codec_info(&amdgpu_dev);
    vbios_info(&amdgpu_dev);
}

fn cache_info(ext_info: &drm_amdgpu_info_device) {
    let gl1_cache_size = ext_info.get_gl1_cache_size();
    let l3_cache_size = ext_info.calc_l3_cache_size_mb();

    println!();
    println!("L1cache (per CU)    : {:4} KiB", ext_info.get_l1_cache_size() / 1024);
    if 0 < gl1_cache_size {
        println!("GL1cache (per SA/SH): {gl1_cache_size:4} KiB");
    }
    println!(
        "L2cache             : {:4} KiB ({} Banks)",
        ext_info.calc_l2_cache_size() / 1024,
        ext_info.num_tcc_blocks
    );
    if 0 < l3_cache_size {
        println!("L3cache             : {l3_cache_size:4} MiB");
    }
}

fn pci_info(pci_bus: &PCI::BUS_INFO) {
    let link = pci_bus.get_link_info(PCI::STATUS::Max);
    println!();
    println!("PCI (domain:bus:dev.func): {pci_bus}");
    println!("PCI Link                 : Gen{}x{}", link.gen, link.width);
}

fn hw_ip_info(amdgpu_dev: &DeviceHandle) {
    let ip_list = [
        HW_IP_TYPE::GFX,
        HW_IP_TYPE::COMPUTE,
        HW_IP_TYPE::DMA,
        HW_IP_TYPE::UVD,
        HW_IP_TYPE::VCE,
        HW_IP_TYPE::UVD_ENC,
        HW_IP_TYPE::VCN_DEC,
        HW_IP_TYPE::VCN_ENC,
        HW_IP_TYPE::VCN_JPEG,
    ];

    println!();
    println!("Hardware IP info:");

    for ip_type in &ip_list {
        if let (Ok(ip_info), Ok(ip_count)) = (
            amdgpu_dev.query_hw_ip_info(*ip_type, 0),
            amdgpu_dev.query_hw_ip_count(*ip_type),
        ) {
            let (major, minor) = ip_info.version();
            let queues = ip_info.num_queues();

            if queues == 0 {
                continue;
            }

            println!(
                "    {ip_type:8} count: {ip_count}, ver: {major:2}.{minor}, queues: {queues}",
                ip_type = ip_type.to_string(),
            );
        }
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

    println!();
    println!("Firmware info:");

    for fw_type in &fw_list {
        let fw_info = match amdgpu_dev.query_firmware_version(*fw_type, 0, 0) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let (ver, ftr) = (fw_info.version, fw_info.feature);

        if ver == 0 {
            continue;
        }

        println!(
            "    {fw_type:<8} feature: {ftr:>3}, ver: {ver:>#10X}",
            fw_type = fw_type.to_string(),
        );
    }
}

fn vbios_info(amdgpu_dev: &DeviceHandle) {
    if let Ok(vbios) = amdgpu_dev.get_vbios_info() {
        println!();
        println!("VBIOS info:");
        println!("    name   : [{}]", vbios.name);
        println!("    pn     : [{}]", vbios.pn);
        println!("    ver_str: [{}]", vbios.ver);
        println!("    date   : [{}]", vbios.date);
    }
}

fn codec_info(amdgpu_dev: &DeviceHandle) {
    if let [Ok(dec), Ok(enc)] = [
        amdgpu_dev.get_video_caps(CAP_TYPE::DECODE),
        amdgpu_dev.get_video_caps(CAP_TYPE::ENCODE),
    ] {
        let codec_list = [
            CODEC::MPEG2,
            CODEC::MPEG4,
            CODEC::VC1,
            CODEC::MPEG4_AVC,
            CODEC::HEVC,
            CODEC::JPEG,
            CODEC::VP9,
            CODEC::AV1,
        ];

        println!();
        println!("Video caps (WIDTHxHEIGHT):");

        for codec in &codec_list {
            let [dec_cap, enc_cap] = [dec, enc].map(|type_| type_.get_codec_info(*codec));
            let dec = format!("{}x{}", dec_cap.max_width, dec_cap.max_height);
            let enc = format!("{}x{}", enc_cap.max_width, enc_cap.max_height);

            println!("{:-^50}", codec.to_string());
            println!(
                concat!(
                    "    Decode: {dec:>10},",
                    "    Encode: {enc:>10}",
                ),
                dec = dec,
                enc = enc,
            );
        }
    }
}
