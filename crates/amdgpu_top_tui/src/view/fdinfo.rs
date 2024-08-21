use std::fmt::{self, Write};
use crate::Opt;

use libamdgpu_top::stat::{FdInfoStat, FdInfoSortType};

/// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_fdinfo.c

const PROC_NAME_LEN: usize = 16;
const PID_MAX_LEN: usize = 7; // 2^22

const VRAM_LABEL: &str = "VRAM";
const GTT_LABEL: &str = "GTT";
const CPU_LABEL: &str = "CPU";
const GFX_LABEL: &str = "GFX";
const COMPUTE_LABEL: &str = "COMP";
const DMA_LABEL: &str = "DMA";
const DEC_LABEL: &str = "DEC";
const ENC_LABEL: &str = "ENC";
const VCN_LABEL: &str = "VCN";
const VPE_LABEL: &str = "VPE";
const KFD_LABEL: &str = "KFD";
// const UVD_ENC_LABEL: &str = "UVD (ENC)";
// const JPEG_LABEL: &str = "JPEG";

use crate::AppTextView;

impl AppTextView {
    pub fn print_fdinfo(
        &mut self,
        stat: &mut FdInfoStat,
        sort: FdInfoSortType,
        reverse: bool,
    ) -> Result<(), fmt::Error> {
        self.text.clear();

        write!(
            self.text.buf,
            " {proc_name:<PROC_NAME_LEN$}|{pid:^PID_MAX_LEN$}|{KFD_LABEL}|{VRAM_LABEL:^6}|{GTT_LABEL:^6}|{CPU_LABEL:^4}|{GFX_LABEL:^4}|{COMPUTE_LABEL:^4}|{DMA_LABEL:^4}",
            proc_name = "Name",
            pid = "PID",
        )?;

        if stat.has_vcn_unified {
            write!(self.text.buf, "|{VCN_LABEL:^4}|")?;
        } else {
            write!(self.text.buf, "|{DEC_LABEL:^4}|{ENC_LABEL:^4}|")?;
        }

        if stat.has_vpe {
            write!(self.text.buf, "|{VPE_LABEL:^4}|")?;
        }

        writeln!(self.text.buf)?;

        stat.sort_proc_usage(sort, reverse);

        self.print_fdinfo_usage(stat)?;

        Ok(())
    }

    pub fn print_fdinfo_usage(&mut self, stat: &FdInfoStat) -> Result<(), fmt::Error> {
        for pu in &stat.proc_usage {
            let utf16_count = pu.name.encode_utf16().count();
            let name_len = if pu.name.len() != utf16_count {
                PROC_NAME_LEN - utf16_count
            } else {
                PROC_NAME_LEN
            };
            write!(
                self.text.buf,
                " {name:name_len$}|{pid:>PID_MAX_LEN$}|{kfd:^3}|{vram:>5}M|{gtt:>5}M|",
                name = pu.name,
                pid = pu.pid,
                kfd = if pu.is_kfd_process { "Y" } else { "" },
                vram = pu.usage.vram_usage >> 10,
                gtt = pu.usage.gtt_usage >> 10,
            )?;

            write!(self.text.buf, "{:>3}%|", pu.cpu_usage)?;

            for (usage, label_len) in [
                (pu.usage.gfx, GFX_LABEL.len()),
                (pu.usage.compute, COMPUTE_LABEL.len()-1),
                (pu.usage.dma, DMA_LABEL.len()),
            ] {
                write!(self.text.buf, "{usage:>label_len$}%|")?;
            }

            if stat.has_vcn_unified {
                write!(self.text.buf, "{:>3}%|", pu.usage.media)?;
            } else {
                write!(self.text.buf, "{:>3}%|", pu.usage.total_dec)?;
                write!(self.text.buf, "{:>3}%|", pu.usage.total_enc)?;
            }

            if stat.has_vpe {
                write!(self.text.buf, "{:>3}%|", pu.usage.vpe)?;
            }

            writeln!(self.text.buf)?;
        }

        Ok(())
    }

    pub fn cb(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo ^= true;
        }
    }

    pub fn cb_reverse_sort(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.reverse_sort ^= true;
        }
    }

    pub fn cb_sort_by_pid(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::PID;
        }
    }

    pub fn cb_sort_by_vram(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::VRAM;
        }
    }

    pub fn cb_sort_by_cpu(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::CPU;
        }
    }

    pub fn cb_sort_by_gfx(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::GFX;
        }
    }

    pub fn cb_sort_by_media(siv: &mut cursive::Cursive) {
        {
            let mut opt = siv.user_data::<Opt>().unwrap().lock().unwrap();
            opt.fdinfo_sort = FdInfoSortType::MediaEngine;
        }
    }
}
