use libamdgpu_top::{
    DevicePath,
    stat::{self, FdInfoStat, ProcInfo},
};

pub fn dump_process(title: &str, list: &[DevicePath]) {
    println!("{title}\n");

    let process_list = stat::get_process_list();

    for device_path in list {
        let Ok(amdgpu_dev) = device_path.init() else { continue };
        let Ok(memory_info) = amdgpu_dev.memory_info() else { continue };

        let mut proc_index: Vec<ProcInfo> = Vec::new();

        stat::update_index_by_all_proc(
            &mut proc_index,
            &[&device_path.render, &device_path.card],
            &process_list,
        );

        let mut fdinfo = FdInfoStat::default();

        fdinfo.get_all_proc_usage(&proc_index);
        fdinfo.sort_proc_usage(Default::default(), false);

        let total_vram_mib = memory_info.vram.total_heap_size >> 20;
        let total_gtt_mib = memory_info.gtt.total_heap_size >> 20;

        println!(
            "{} ({}), VRAM {:5}/{:5} MiB, GTT {:5}/{:5} MiB",
            device_path.pci,
            device_path.device_name,
            memory_info.vram.heap_usage >> 20,
            total_vram_mib,
            memory_info.gtt.heap_usage >> 20,
            total_gtt_mib,
        );

        for pu in fdinfo.proc_usage {
            let usage_vram_mib = pu.usage.vram_usage >> 10; // KiB -> MiB
            let usage_gtt_mib = pu.usage.gtt_usage >> 10; // KiB -> MiB

            println!(
                "    {:15} ({:7}), ctxs {:2}, VRAM {:5} MiB ({:3}%), GTT {:5} MiB ({:3}%)",
                pu.name,
                pu.pid,
                pu.ids_count,
                usage_vram_mib,
                (usage_vram_mib * 100) / total_vram_mib,
                usage_gtt_mib,
                (usage_gtt_mib * 100) / total_gtt_mib,
            );

            println!(
                "{:28} Requested: VRAM {:5} MiB, {:6} GTT {:5} MiB",
                "",
                pu.usage.amd_requested_vram >> 10,
                "",
                pu.usage.amd_requested_gtt >> 10,
            );

            println!(
                "{:28}   Evicted: VRAM {:5} MiB",
                "",
                pu.usage.amd_evicted_vram >> 10,
            );
        }

        println!();
    }
}
