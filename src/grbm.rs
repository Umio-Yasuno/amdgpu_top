/* GRBM: Graphics Register Block, Graphics Register Bus Manager? */
/* ref: https://github.com/freedesktop/mesa-r600_demo/blob/master/r600_lib.c */
use crate::util::{BITS, Text};

#[derive(Default)]
pub struct GRBM {
    pub flag: bool,
    pub is_gfx10_plus: bool,
    // pub bits: GRBM_BITS,
    pub bits: BITS,
    pub text: Text,
}

impl GRBM {
    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        if !self.flag {
            return;
        }

        if !self.is_gfx10_plus {
            write!(
                self.text.buf,
                concat!(
                    " {vgt_name:<30} => {vgt:3}%,",
                    " {ia_name:<30} => {ia:3}% \n",
                ),
                vgt_name = "Vertex Grouper / Tessellator",
                vgt = self.bits.0[17],
                ia_name = "Input Assembly",
                ia = self.bits.0[19],
            )
            .unwrap();
        }

        let wd_ge_name = if self.is_gfx10_plus {
            "Geometry Engine"
        } else {
            "Work Distributor"
        };

        write!(
            self.text.buf,
            concat!(
                " {ta_name:<30 } => {ta:3}%,",
                " {sx_name:<30 } => {sx:3}% \n",
                " {spi_name:<30} => {spi:3}%,",
                " {pa_name:<30 } => {pa:3}% \n",
                " {db_name:<30 } => {db:3}%,",
                " {cb_name:<30 } => {cb:3}% \n",
                " {cp_name:<30 } => {cp:3}%,",
                " {gui_name:<30} => {gui:3}% \n",
                " {wd_ge_name:<30} => {wd_ge:3}%,",
                " {gds_name:<30} => {gds:3}% \n",
            ),
            ta_name = "Texture Pipe",
            ta = self.bits.0[14],
            sx_name = "Shader Export",
            sx = self.bits.0[20],
            spi_name = "Shader Processor Interpolator",
            spi = self.bits.0[22],
            pa_name = "Primitive Assembly",
            pa = self.bits.0[25],
            db_name = "Depth Block",
            db = self.bits.0[26],
            cb_name = "Color Block",
            cb = self.bits.0[30],
            cp_name = "Command Processor",
            cp = self.bits.0[29],
            gui_name = "Graphics Pipe",
            gui = self.bits.0[31],
            wd_ge_name = wd_ge_name,
            wd_ge = self.bits.0[21],
            gds_name = "Global Data Share",
            gds = self.bits.0[15],
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
