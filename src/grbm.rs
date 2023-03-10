/* GRBM: Graphics Register Block, Graphics Register Bus Manager? */
/* ref: https://github.com/freedesktop/mesa-r600_demo/blob/master/r600_lib.c */
use crate::util::Text;
use super::get_bit;

#[derive(Default)]
#[allow(non_camel_case_types)]
pub struct GRBM_BITS {
    ta: u8, // Texture Pipe, Texture Addresser?
    gds: u8, // Global Data Share
    vgt: u8, // Vertex Grouper and Tessellator
    ia: u8, // Input Assembly?
    sx: u8, // Shader Export
    wd_ge: u8, // Work Distributor, Geometry Engine? from GFX10
    spi: u8, // Shader Pipe Interpolator
    bci: u8, // Barycentric interpolation control
    sc: u8, // Scan Convertor
    pa: u8, // Primitive Assembly
    db: u8, // Depth Block/Buffer
    cp: u8, // Command Processor?
    cb: u8, // Color Block/Buffer
    gui_active: u8,
}

impl GRBM_BITS {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn acc(&mut self, reg: u32) {
        self.ta += get_bit!(reg, 14);
        self.gds += get_bit!(reg, 15);
        self.vgt += get_bit!(reg, 17);
        self.ia += get_bit!(reg, 19);
        self.sx += get_bit!(reg, 20);
        self.wd_ge += get_bit!(reg, 21);
        self.spi += get_bit!(reg, 22);
        self.bci += get_bit!(reg, 23);
        self.sc += get_bit!(reg, 24);
        self.pa += get_bit!(reg, 25);
        self.db += get_bit!(reg, 26);
        self.cp += get_bit!(reg, 29);
        self.cb += get_bit!(reg, 30);
        self.gui_active += get_bit!(reg, 31);
    }
}

#[derive(Default)]
pub struct GRBM {
    pub flag: bool,
    pub bits: GRBM_BITS,
    pub text: Text,
}

impl GRBM {
    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        if !self.flag {
            return;
        }

        write!(
            self.text.buf,
            concat!(
                " {ta_name:<30 } => {ta:3}%,",
                " {vgt_name:<30} => {vgt:3}% \n",
                " {ia_name:<30 } => {ia:3}%,",
                " {sx_name:<30 } => {sx:3}% \n",
                " {spi_name:<30} => {spi:3}%,",
                " {pa_name:<30 } => {pa:3}% \n",
                " {db_name:<30 } => {db:3}%,",
                " {cb_name:<30 } => {cb:3}% \n",
                " {cp_name:<30 } => {cp:3}%,",
                " {gui_name:<30} => {gui:3}% \n",
                " {wd_ge_name:<30} => {wd_ge}%",
            ),
            ta_name = "Texture Pipe",
            ta = self.bits.ta,
            vgt_name = "Vertex Grouper / Tessellator",
            vgt = self.bits.vgt,
            ia_name = "Input Assembly",
            ia = self.bits.ia,
            sx_name = "Shader Export",
            sx = self.bits.sx,
            spi_name = "Shader Processor Interpolator",
            spi = self.bits.spi,
            pa_name = "Primitive Assembly",
            pa = self.bits.pa,
            db_name = "Depth Block",
            db = self.bits.db,
            cb_name = "Color Block",
            cb = self.bits.cb,
            cp_name = "Command Processor",
            cp = self.bits.cp,
            gui_name = "Graphics Pipe",
            gui = self.bits.gui_active,
            wd_ge_name = "Work Distributor / Geometry Engine",
            wd_ge = self.bits.wd_ge,
        )
        .unwrap();
    }

    pub fn dump(&mut self) {
        self.print();
        self.text.set();
        self.bits.clear();
    }
}
