use crate::bindings::{inl, outl};

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// Construct a PCI address based on bus, device, function, and offset.
pub fn pci_config_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    (1 << 31)               // Enable bit
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC) // Mask to ensure 32-bit alignment
}

pub unsafe fn pci_read_config(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = pci_config_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS, address);
    inl(PCI_CONFIG_DATA)
}

pub unsafe fn pci_write_config(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address = pci_config_address(bus, device, function, offset);
    outl(PCI_CONFIG_ADDRESS, address);
    outl(PCI_CONFIG_DATA, value);
}

pub fn find_base_address(vendor_id: u32, device_id: u32) -> Option<*mut u8> {
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                let vendor_id_read =
                    unsafe { pci_read_config(bus, device, function, 0x00) } & 0xFFFF;
                let device_id_read =
                    (unsafe { pci_read_config(bus, device, function, 0x00) } >> 16) & 0xFFFF;
                if vendor_id_read == vendor_id && (device_id_read & device_id) == device_id {
                    let bar0 = unsafe { pci_read_config(bus, device, function, 0x10) };
                    if bar0 & 0x1 == 0 {
                        return Some((bar0 & 0xFFFFFFF0) as *mut u8);
                    }
                }
            }
        }
    }
    None
}
