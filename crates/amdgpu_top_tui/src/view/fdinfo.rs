use std::fmt::{self, Write};
use super::Text;
use crate::Opt;
use std::time::Duration;

use libamdgpu_top::stat::{sort_proc_usage, ProcInfo, FdInfoStat, FdInfoSortType};

/// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_fdinfo.c

const PROC_NAME_LEN: usize = 16;

const VRAM_LABEL: &str = "VRAM";
const GTT_LABEL: &str = "GTT";
const CPU_LABEL: &str = "CPU";
const GFX_LABEL: &str = "GFX";
const COMPUTE_LABEL: &str = "COMP";
const DMA_LABEL: &str = "DMA";
const DEC_LABEL: &str = "DEC";
const ENC_LABEL: &str = "ENC";
const VCN_LABEL: &str = "VCN";
// const UVD_ENC_LABEL: &str = "UVD (ENC)";
// const JPEG_LABEL: &str = "JPEG";

#[derive(Clone, Default)]
pub struct FdInfoView {
    pub stat: FdInfoStat,
    pub has_vcn_unified: bool,
    pub text: Text,
}

impl FdInfoView {
    pub fn new(interval: Duration, has_vcn: bool, has_vcn_unified: bool) -> Self {
        let stat = FdInfoStat { interval, has_vcn, has_vcn_unified, ..Default::default() };

        Self {
            stat,
            has_vcn_unified,
            ..Default::default()
        }
    }

    pub fn print(
        &mut self,
        proc_index: &[ProcInfo],
        sort: &FdInfoSortType,
        reverse: bool
    ) -> Result<(), fmt::Error> {
        self.text.clear();

        write!(
            self.text.buf,
            " {pad:25} |{VRAM_LABEL:^6}|{GTT_LABEL:^6}|{CPU_LABEL:^4}|{GFX_LABEL:^4}|{COMPUTE_LABEL:^4}|{DMA_LABEL:^4}",
            pad = "",
        )?;

        if self.has_vcn_unified {
            writeln!(self.text.buf, "|{VCN_LABEL:^4}|")?;
        } else {
            writeln!(self.text.buf, "|{DEC_LABEL:^4}|{ENC_LABEL:^4}|")?;
        }

        self.stat.get_all_proc_usage(proc_index);

        sort_proc_usage(&mut self.stat.proc_usage, sort, reverse);

        self.print_usage()?;

        Ok(())
    }

    pub fn print_usage(&mut self) -> Result<(), fmt::Error> {
        for pu in &self.stat.proc_usage {
            let utf16_count = pu.name.encode_utf16().count();
            let name_len = if pu.name.len() != utf16_count {
                PROC_NAME_LEN - utf16_count
            } else {
                PROC_NAME_LEN
            };
            write!(
                self.text.buf,
                " {name:name_len$}({pid:>8})|{vram:>5}M|{gtt:>5}M|",
                name = pu.name,
                pid = pu.pid,
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

            if self.has_vcn_unified {
                write!(self.text.buf, "{:>3}%|", pu.usage.media)?;
            } else {
                let dec_usage = pu.usage.dec + pu.usage.vcn_jpeg; // UVD/VCN/VCN_JPEG
                let enc_usage = pu.usage.enc + pu.usage.uvd_enc; // UVD/VCN
                write!(self.text.buf, "{dec_usage:>3}%|")?;
                write!(self.text.buf, "{enc_usage:>3}%|")?;
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
