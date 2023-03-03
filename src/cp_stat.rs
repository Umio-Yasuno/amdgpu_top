macro_rules! get_bit {
    ($reg: expr, $shift: expr) => {
        (($reg >> $shift) & 0b1) as u8
    };
}

pub struct CP_STAT {
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

impl CP_STAT {
    pub const fn new() -> Self {
        Self {
            pfp: 0,
            // meq: 0,
            me: 0,
            // query: 0,
            surface_sync: 0,
            dma: 0,
            scratch_memory: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new()
    }

    pub fn acc(&mut self, reg: u32) {
        self.pfp += get_bit!(reg, 15);
        // self.meq += get_bit!(reg, 16);
        self.me += get_bit!(reg, 17);
        self.surface_sync += get_bit!(reg, 21);
        self.dma += get_bit!(reg, 22);
        self.scratch_memory += get_bit!(reg, 24);
    }

    pub fn _stat(&self) -> String {
        format!(
            concat!(
                "PFP_BUSY:       {pfp:3}%\n",
                "ME_BUSY:        {me:3}%\n",
                "SURFACE_SYNC:   {surface_sync:3}%\n",
                "DMA_BUSY:       {dma:3}%\n",
                "SCRATCH_MEMORY: {scratch_memory:3}%\n",
            ),
            pfp = self.pfp,
            me = self.me,
            surface_sync = self.surface_sync,
            dma = self.dma,
            scratch_memory = self.scratch_memory,
        )
    }

    pub fn verbose_stat(&self) -> String {
        format!(
            concat!(
                " {pfp_name:<30           } => {pfp:3}% \n",
                " {me_name:<30            } => {me:3}% \n",
                " {surface_sync_name:<30  } => {surface_sync:3}% \n",
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
