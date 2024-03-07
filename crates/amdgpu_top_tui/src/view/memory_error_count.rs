use std::fmt::{self, Write};
use libamdgpu_top::AMDGPU::RasErrorCount;

use crate::AppTextView;

impl AppTextView {
    pub fn print_memory_error_count(&mut self, ecc: &RasErrorCount) -> Result<(), fmt::Error> {
        self.text.clear();

        writeln!(
            self.text.buf,
            " Corrected: {:>4},  UnCorrected: {:>4}",
            ecc.corrected,
            ecc.uncorrected,
        )?;

        Ok(())
    }
}
