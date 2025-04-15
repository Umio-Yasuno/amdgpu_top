use super::FdInfoStat;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum FdInfoSortType {
    PID,
    KFD,
    #[default]
    VRAM,
    GTT,
    CPU,
    GFX,
    Compute,
    DMA, // SDMA, System DMA Engine
    Decode,
    Encode,
    MediaEngine,
    VPE, // Video Processing Engine
    VCNU, // VCN Unified
}

impl FdInfoStat {
    pub fn sort_proc_usage(&mut self, sort: FdInfoSortType, reverse: bool) {
        self.proc_usage.sort_by(|a, b|
            match (sort, reverse) {
                (FdInfoSortType::PID, false) => b.pid.cmp(&a.pid),
                (FdInfoSortType::PID, true) => a.pid.cmp(&b.pid),
                (FdInfoSortType::KFD, false) => b.is_kfd_process.cmp(&a.is_kfd_process),
                (FdInfoSortType::KFD, true) => a.is_kfd_process.cmp(&b.is_kfd_process),
                (FdInfoSortType::VRAM, false) => b.usage.vram_usage.cmp(&a.usage.vram_usage),
                (FdInfoSortType::VRAM, true) => a.usage.vram_usage.cmp(&b.usage.vram_usage),
                (FdInfoSortType::GTT, false) => b.usage.gtt_usage.cmp(&a.usage.gtt_usage),
                (FdInfoSortType::GTT, true) => a.usage.gtt_usage.cmp(&b.usage.gtt_usage),
                (FdInfoSortType::CPU, false) => b.usage.cpu.cmp(&a.usage.cpu),
                (FdInfoSortType::CPU, true) => a.usage.cpu.cmp(&b.usage.cpu),
                (FdInfoSortType::GFX, false) => b.usage.gfx.cmp(&a.usage.gfx),
                (FdInfoSortType::GFX, true) => a.usage.gfx.cmp(&b.usage.gfx),
                (FdInfoSortType::Compute, false) => b.usage.gfx.cmp(&a.usage.compute),
                (FdInfoSortType::Compute, true) => a.usage.gfx.cmp(&b.usage.compute),
                (FdInfoSortType::DMA, false) => b.usage.gfx.cmp(&a.usage.dma),
                (FdInfoSortType::DMA, true) => a.usage.gfx.cmp(&b.usage.dma),
                (FdInfoSortType::Decode, false) => b.usage.total_dec.cmp(&a.usage.total_dec),
                (FdInfoSortType::Decode, true) => a.usage.total_dec.cmp(&b.usage.total_dec),
                (FdInfoSortType::Encode, false) => b.usage.total_enc.cmp(&a.usage.total_enc),
                (FdInfoSortType::Encode, true) => a.usage.total_enc.cmp(&b.usage.total_enc),
                (FdInfoSortType::MediaEngine, false) => b.usage.media.cmp(&a.usage.media),
                (FdInfoSortType::MediaEngine, true) => a.usage.media.cmp(&b.usage.media),
                (FdInfoSortType::VPE, false) => b.usage.media.cmp(&a.usage.vpe),
                (FdInfoSortType::VPE, true) => a.usage.media.cmp(&b.usage.vpe),
                (FdInfoSortType::VCNU, false) => b.usage.media.cmp(&a.usage.vcn_unified),
                (FdInfoSortType::VCNU, true) => a.usage.media.cmp(&b.usage.vcn_unified),
            }
        );
    }
}
