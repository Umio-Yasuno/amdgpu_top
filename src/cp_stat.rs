#[allow(non_camel_case_types)]
pub struct CP_STAT {
    pub flag: bool,
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
    // ce: u8, // ?
    // cp: u8, // Command Processor
}

impl Default for CP_STAT {
    fn default() -> Self {
        Self {
            flag: false,
            pfp: 0,
            // meq: 0,
            me: 0,
            // query: 0,
            surface_sync: 0,
            dma: 0,
            scratch_memory: 0,
        }
    }
}

impl CP_STAT {
    pub fn clear(&mut self) {
        self.pfp = 0;
        self.me = 0;
        self.surface_sync = 0;
        self.dma = 0;
        self.scratch_memory = 0;
    }

    pub fn acc(&mut self, reg: u32) {
        self.pfp += ((reg >> 15) & 0b1) as u8;
        // self.meq += get_bit!(reg, 16);
        self.me += ((reg >> 17) & 0b1) as u8;
        self.surface_sync += ((reg >> 21) & 0b1) as u8;
        self.dma += ((reg >> 22) & 0b1) as u8;
        self.scratch_memory += ((reg >> 24) & 0b1) as u8;
    }

    pub fn stat(&self) -> String {
        if !self.flag {
            return "".to_string();
        }

        format!(
            concat!(
                " {pfp_name:<30           } => {pfp:3}%,",
                " {me_name:<30            } => {me:3}% \n",
                " {surface_sync_name:<30  } => {surface_sync:3}%,",
                " {dma_name:<30           } => {dma:3}% \n",
                " {scratch_memory_name:<30} => {scratch_memory:3}% \n",
            ),
            pfp_name = "Prefetch Parser",
            pfp = self.pfp,
            me_name = "Micro Engine",
            me = self.me,
            surface_sync_name = "Surface Sync",
            surface_sync = self.surface_sync,
            dma_name = "DMA",
            dma = self.dma,
            scratch_memory_name = "Scratch Memory",
            scratch_memory = self.scratch_memory,
        )
    }
}
