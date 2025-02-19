pub const PANEL_WIDTH: usize = 70;
pub const PC_BAR_WIDTH: usize = 35;
pub const VRAM_LABEL_WIDTH: usize = 6;

mod fdinfo;
mod xdna_fdinfo;
// pub use fdinfo::*;

mod gpu_metrics;
// pub use gpu_metrics::*;

mod perf_counter;
pub use perf_counter::*;

mod sensors;
// pub use sensors::*;

mod util;
pub use util::*;

mod vram;
pub use vram::*;

mod activity;
pub use activity::*;

mod memory_error_count;

#[derive(Clone, Default)]
pub(crate) struct AppTextView {
    pub text: Text,
}
