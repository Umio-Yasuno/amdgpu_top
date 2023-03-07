/* GRBM: Graphics Register Block */

macro_rules! get_bit {
    ($reg: expr, $shift: expr) => {
        (($reg >> $shift) & 0b1) as u8
    };
}

pub struct GRBM {
    pub flag: bool,
    ta: u8, // Texture Addresser?
    gds: u8, // Global Data Share
    vgt: u8, // Vertex Grouper and Tessellator
    ia: u8, // Input Assembly?
    sx: u8, // Shader Export
    spi: u8, // Shader Pipe Interpolator
    bci: u8, // Barycentric interpolation control
    sc: u8, // Scan Convertor
    pa: u8, // Primitive Assembly
    db: u8, // Depth Block/Buffer
    cp: u8, // Command Processor?
    cb: u8, // Color Block/Buffer
    gui_active: u8,
}

impl Default for GRBM {
    fn default() -> Self {
        Self {
            flag: true,
            ta: 0,
            gds: 0,
            vgt: 0,
            ia: 0,
            sx: 0,
            spi: 0,
            bci: 0,
            sc: 0,
            pa: 0,
            db: 0,
            cp: 0,
            cb: 0,
            gui_active: 0,
        }
    }
}

impl GRBM {
    pub fn clear(&mut self) {
        self.ta = 0;
        self.gds = 0;
        self.vgt = 0;
        self.ia = 0;
        self.sx = 0;
        self.spi = 0;
        self.bci = 0;
        self.sc = 0;
        self.pa = 0;
        self.db = 0;
        self.cp = 0;
        self.cb = 0;
        self.gui_active = 0;
    }

    pub fn acc(&mut self, reg: u32) {
        self.ta += get_bit!(reg, 14);
        self.gds += get_bit!(reg, 15);
        self.vgt += get_bit!(reg, 17);
        self.ia += get_bit!(reg, 19);
        self.sx += get_bit!(reg, 20);
        self.spi += get_bit!(reg, 22);
        self.bci += get_bit!(reg, 23);
        self.sc += get_bit!(reg, 24);
        self.pa += get_bit!(reg, 25);
        self.db += get_bit!(reg, 26);
        self.cp += get_bit!(reg, 29);
        self.cb += get_bit!(reg, 30);
        self.gui_active += get_bit!(reg, 31);
    }

    pub fn stat(&self) -> String {
        if !self.flag {
            return "".to_string();
        }

        format!(
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
            ),
            ta_name = "Texture Addresser",
            ta = self.ta,
            vgt_name = "Vertex Grouper / Tessellator",
            vgt = self.vgt,
            ia_name = "Input Assembly",
            ia = self.ia,
            sx_name = "Shader Export",
            sx = self.sx,
            spi_name = "Shader Processor Interpolator",
            spi = self.spi,
            pa_name = "Primitive Assembly",
            pa = self.pa,
            db_name = "Depth Block",
            db = self.db,
            cb_name = "Color Block",
            cb = self.cb,
            cp_name = "Command Processor",
            cp = self.cp,
            gui_name = "GUI Active",
            gui = self.gui_active,
        )
    }
}
