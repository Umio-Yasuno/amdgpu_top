use super::get_bit;
use crate::util::Text;

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct CP_STAT_BITS {
    // dc: u8, // Data Cache?
    pfp: u8, // Prefetch Parser
    // meq: u8, // Micro Engine Queue?
    me: u8, // Micro Engine
    // query: u8,
    // semaphore: u8,
    // interrupt: u8,
    surface_sync: u8,
    dma: u8,
    // rciu: u8, // ?
    scratch_memory: u8, // LocalDataShare?
    // cpc_cpg: u8, // Command Processor Compute/Graphics?
    // cpf: u8, // Command Processor Fetchers
    // ce: u8, // Constant Engine?
    // cp: u8, // Command Processor
}

impl CP_STAT_BITS {
    pub fn clear(&mut self) {
        *self = Self::default()
    }

    pub fn acc(&mut self, reg: u32) {
        self.pfp += get_bit!(reg, 15);
        // self.meq += get_bit!(reg, 16);
        self.me += get_bit!(reg, 17);
        self.surface_sync += get_bit!(reg, 21);
        self.dma += get_bit!(reg, 22);
        self.scratch_memory += get_bit!(reg, 24);
    }
}

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct CP_STAT {
    pub flag: bool,
    pub bits: CP_STAT_BITS,
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
            pfp = self.bits.pfp,
            me_name = "Micro Engine",
            me = self.bits.me,
            surface_sync_name = "Surface Sync",
            surface_sync = self.bits.surface_sync,
            dma_name = "DMA",
            dma = self.bits.dma,
            scratch_memory_name = "Scratch Memory",
            scratch_memory = self.bits.scratch_memory,
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
