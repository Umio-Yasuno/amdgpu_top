use libdrm_amdgpu_sys::PCI;

#[allow(non_camel_case_types)]
pub(crate) struct PCI_LINK_INFO {
    pub(crate) cur: PCI::LINK,
    pub(crate) max: PCI::LINK,
    pub(crate) buf: String,
}

impl PCI_LINK_INFO {
    pub(crate) fn new(pci_bus: &PCI::BUS_INFO) -> Self {
        Self {
            cur: pci_bus.get_link_info(PCI::STATUS::Current),
            max: pci_bus.get_link_info(PCI::STATUS::Max),
            buf: String::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.buf.clear()
    }

    pub(crate) fn update_print(&mut self, pci_bus: &PCI::BUS_INFO) {
        self.update(pci_bus);
        self.print(pci_bus);
    }

    pub(crate) fn update(&mut self, pci_bus: &PCI::BUS_INFO) {
        self.cur = pci_bus.get_link_info(PCI::STATUS::Current)
    }

    pub(crate) fn print(&mut self, pci_bus: &PCI::BUS_INFO) {
        use std::fmt::Write;

        self.clear();

        writeln!(
            self.buf,
            " PCI ({pci_bus}): Gen{cur_gen}x{cur_width} @ Gen{max_gen}x{max_width} (Max) ",
            pci_bus = pci_bus,
            cur_gen = self.cur.gen,
            cur_width = self.cur.width,
            max_gen = self.max.gen,
            max_width = self.max.width,
        ).unwrap();
    }
}
