use crate::util::Text;
use libdrm_amdgpu_sys::PCI;

#[allow(non_camel_case_types)]
pub struct PCI_LINK_INFO {
    cur: PCI::LINK,
    max: PCI::LINK,
    bus_info: PCI::BUS_INFO,
    pub text: Text,
}

impl PCI_LINK_INFO {
    pub fn new(pci_bus: &PCI::BUS_INFO) -> Self {
        Self {
            cur: pci_bus.get_link_info(PCI::STATUS::Current),
            max: pci_bus.get_link_info(PCI::STATUS::Max),
            bus_info: pci_bus.clone(),
            text: Text::default(),
        }
    }

    pub fn update_status(&mut self) {
        self.cur = self.bus_info.get_link_info(PCI::STATUS::Current)
    }

    pub fn print(&mut self) {
        use std::fmt::Write;

        self.text.clear();

        write!(
            self.text.buf,
            " PCI ({pci_bus}): Gen{cur_gen}x{cur_width:<2} @ Gen{max_gen}x{max_width:<2} (Max) ",
            pci_bus = self.bus_info,
            cur_gen = self.cur.gen,
            cur_width = self.cur.width,
            max_gen = self.max.gen,
            max_width = self.max.width,
        )
        .unwrap();
    }
}
