// for debug

use libamdgpu_top::{stat, xdna};

pub fn dump_xdna_device() {
    let Some(xdna_device) = xdna::find_xdna_device() else {
        println!("There are no the XDNA NPU devices found.");
        return;
    };

    println!("{xdna_device:#?}");

    if let Ok(fw_ver) = xdna_device.get_xdna_fw_version() {
        println!("FW Version: {fw_ver}");
    }

    let mut xdna_proc_index = xdna_device.arc_proc_index.lock().unwrap();

    stat::update_index_by_all_proc(
        &mut xdna_proc_index,
        &[&xdna_device.render, &xdna_device.card],
        &stat::get_all_processes(),
    );

    let mut xdna_fdinfo = xdna::XdnaFdInfoStat::default();
    xdna_fdinfo.get_all_proc_usage(&xdna_proc_index);

    println!("{:#?}", xdna_fdinfo.proc_usage);
}
