use crate::util::{BITS, Text};

#[derive(Default)]
pub struct GRBM2 {
    pub flag: bool,
    pub bits: BITS,
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
            tc = self.bits.0[25],
            cpf_name = "Command Processor Fetcher",
            cpf = self.bits.0[28],
            cpc_name = "Command Processor Compute",
            cpc = self.bits.0[29],
            cpg_name = "Command Processor Graphics",
            cpg = self.bits.0[30],
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
