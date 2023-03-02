macro_rules! get_bit {
    ($reg: expr, $shift: expr) => {
        (($reg >> $shift) & 0b1) as u8
    };
}

pub struct CP_STAT {
    // dc: u8,
    pfp: u8,
    meq: u8,
    me: u8,
    // query: u8,
    // semaphore: u8,
    // interrupt: u8,
    surface_sync: u8,
    dma: u8,
    scratch_memory: u8,
}

impl CP_STAT {
    pub const fn new() -> Self {
        Self {
            pfp: 0,
            meq: 0,
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
        self.meq += get_bit!(reg, 16);
        self.me += get_bit!(reg, 17);
        self.surface_sync += get_bit!(reg, 21);
        self.dma += get_bit!(reg, 22);
        self.scratch_memory += get_bit!(reg, 24);
    }

    pub fn stat(&self) -> String {
        format!(
            concat!(
                "PFP_BUSY:       {pfp:3}%\n",
                "MEQ_BUSY:       {meq:3}%\n",
                "ME_BUSY:        {me:3}%\n",
                "SURFACE_SYNC:   {surface_sync:3}%\n",
                "DMA_BUSY:       {dma:3}%\n",
                "SCRATCH_MEMORY: {scratch_memory:3}%\n",
            ),
            pfp = self.pfp,
            meq = self.meq,
            me = self.me,
            surface_sync = self.surface_sync,
            dma = self.dma,
            scratch_memory = self.scratch_memory,
        )
    }
}
