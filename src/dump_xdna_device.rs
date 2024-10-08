use libamdgpu_top::xdna;

pub fn dump_xdna_device() {
    let xdna_device = xdna::find_xdna_device();

    println!("{xdna_device:#?}");
}
