use std::fmt::{self, Write};
// use crate::Opt;

use libamdgpu_top::xdna::XdnaFdInfoStat;

const PROC_NAME_LEN: usize = 16;
const PID_MAX_LEN: usize = 7; // 2^22

const MEMORY_LABEL: &str = "MEM";
const NPU_LABEL: &str = "NPU";

use crate::AppTextView;

impl AppTextView {
    // pub const XDNA_FDINFO_TITLE: &str = "XDNA fdinfo";

    pub fn print_xdna_fdinfo(
        &mut self,
        stat: &mut XdnaFdInfoStat,
    ) -> Result<(), fmt::Error> {
        self.text.clear();

        write!(
            self.text.buf,
            " {proc_name:<PROC_NAME_LEN$}|{pid:^PID_MAX_LEN$}|{MEMORY_LABEL:^6}|{NPU_LABEL:^4}|",
            proc_name = "Name",
            pid = "PID",
        )?;

        self.print_xdna_fdinfo_usage(stat)?;

        Ok(())
    }

    pub fn print_xdna_fdinfo_usage(&mut self, stat: &XdnaFdInfoStat) -> Result<(), fmt::Error> {
        for pu in &stat.proc_usage {
            if pu.ids_count == 0 {
                continue;
            }

            let utf16_count = pu.name.encode_utf16().count();
            let name_len = if pu.name.len() != utf16_count {
                PROC_NAME_LEN - utf16_count
            } else {
                PROC_NAME_LEN
            };
            write!(
                self.text.buf,
                " {name:name_len$}|{pid:>PID_MAX_LEN$}|{total:>5}M|",
                name = pu.name,
                pid = pu.pid,
                total = pu.usage.total_memory >> 10,
            )?;

            // write!(self.text.buf, "{:>3}%|", pu.cpu_usage)?;

            for (usage, label_len) in [
                (pu.usage.npu, NPU_LABEL.len()),
            ] {
                write!(self.text.buf, "{usage:>label_len$}%|")?;
            }

            writeln!(self.text.buf)?;
        }

        Ok(())
    }
/*
    pub fn xdna_fdinfo_name(index: usize) -> String {
        format!("{} {index}", Self::XDNA_FDINFO_TITLE)
    }
*/
    // TODO: cb
    // No one has tested the functionality for XDNA
}
