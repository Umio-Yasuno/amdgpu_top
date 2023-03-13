use crate::util::{BITS, Text};

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct CP_STAT {
    pub flag: bool,
    pub bits: BITS,
    pub text: Text,
}

impl CP_STAT {
    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        if !self.flag {
            return;
        }

        write!(
            self.text.buf,
            concat!(
                " {pfp_name:<30           } => {pfp:3}%,",
                " {me_name:<30            } => {me:3}% \n",
                " {surface_sync_name:<30  } => {surface_sync:3}%,",
                " {dma_name:<30           } => {dma:3}% \n",
                " {scratch_memory_name:<30} => {scratch_memory:3}% \n",
            ),
            pfp_name = "Prefetch Parser",
            pfp = self.bits.0[15],
            me_name = "Micro Engine",
            me = self.bits.0[17],
            surface_sync_name = "Surface Sync",
            surface_sync = self.bits.0[21],
            dma_name = "DMA",
            dma = self.bits.0[22],
            scratch_memory_name = "Scratch Memory",
            scratch_memory = self.bits.0[24],
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
