use crate::util::Text;
use crate::get_bit;

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct GRBM2_BITS {
    // ea: u8, Efficiency Arbiter, GFX9+
    tc: u8, // Texture Cache, Texture Cache per Pipe (GFX10+) Vector L1 Cache
    cpf: u8, // Command Processor - Fetcher
    cpc: u8, // Command Processor - Compute
    cpg: u8, // Command Processor - Graphics
}

impl GRBM2_BITS {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn acc(&mut self, reg: u32) {
        self.tc += get_bit!(reg, 25);
        self.cpf += get_bit!(reg, 28);
        self.cpc += get_bit!(reg, 29);
        self.cpg += get_bit!(reg, 30);
    }
}

#[derive(Default)]
pub struct GRBM2 {
    pub flag: bool,
    pub bits: GRBM2_BITS,
    pub text: Text,
}

impl GRBM2 {
    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        if !self.flag {
            return;
        }

        write!(
            self.text.buf,
            concat!(
                " {tc_name:<30 } => {tc:3}%,",
                " {cpf_name:<30} => {cpf:3}% \n",
                " {cpc_name:<30} => {cpc:3}%,",
                " {cpg_name:<30} => {cpg:3}% \n",
            ),
            tc_name = "Texture Cache",
            tc = self.bits.tc,
            cpf_name = "Command Processor Fetcher",
            cpf = self.bits.cpf,
            cpc_name = "Command Processor Compute",
            cpc = self.bits.cpc,
            cpg_name = "Command Processor Graphics",
            cpg = self.bits.cpg,
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
