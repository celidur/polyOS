#![allow(dead_code)]

use crate::device::io::{inl, outl};

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

#[derive(Clone, Copy, Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision_id: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PciBar {
    Io(u16),
    Memory(u32),
}

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
    unsafe { outl(PCI_CONFIG_ADDRESS, address) };
    unsafe { inl(PCI_CONFIG_DATA) }
}

pub unsafe fn pci_write_config(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address = pci_config_address(bus, device, function, offset);
    unsafe { outl(PCI_CONFIG_ADDRESS, address) };
    unsafe { outl(PCI_CONFIG_DATA, value) };
}

impl PciDevice {
    pub fn read_config(&self, offset: u8) -> u32 {
        unsafe { pci_read_config(self.bus, self.device, self.function, offset) }
    }

    pub fn write_config(&self, offset: u8, value: u32) {
        unsafe { pci_write_config(self.bus, self.device, self.function, offset, value) };
    }

    pub fn enable_io_space(&self) {
        self.set_command_bits(1 << 0);
    }

    pub fn enable_memory_space(&self) {
        self.set_command_bits(1 << 1);
    }

    pub fn enable_bus_mastering(&self) {
        self.set_command_bits(1 << 2);
    }

    pub fn bar(&self, index: u8) -> Option<PciBar> {
        if index >= 6 {
            return None;
        }

        let raw = self.read_config(0x10 + index * 4);
        if raw == 0 || raw == u32::MAX {
            return None;
        }

        if raw & 0x1 != 0 {
            Some(PciBar::Io((raw & 0xFFFC) as u16))
        } else {
            Some(PciBar::Memory(raw & 0xFFFFFFF0))
        }
    }

    pub fn interrupt_line(&self) -> u8 {
        (self.read_config(0x3C) & 0xFF) as u8
    }

    fn set_command_bits(&self, bits: u16) {
        let command = self.read_config(0x04);
        self.write_config(0x04, command | bits as u32);
    }
}

fn read_device(bus: u8, device: u8, function: u8) -> Option<PciDevice> {
    let id = unsafe { pci_read_config(bus, device, function, 0x00) };
    let vendor_id = (id & 0xFFFF) as u16;
    if vendor_id == 0xFFFF {
        return None;
    }

    let class = unsafe { pci_read_config(bus, device, function, 0x08) };
    Some(PciDevice {
        bus,
        device,
        function,
        vendor_id,
        device_id: ((id >> 16) & 0xFFFF) as u16,
        class_code: ((class >> 24) & 0xFF) as u8,
        subclass: ((class >> 16) & 0xFF) as u8,
        prog_if: ((class >> 8) & 0xFF) as u8,
        revision_id: (class & 0xFF) as u8,
    })
}

pub fn find_device(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                if let Some(pci_device) = read_device(bus, device, function) {
                    if pci_device.vendor_id == vendor_id && pci_device.device_id == device_id {
                        return Some(pci_device);
                    }
                }
            }
        }
    }
    None
}

pub fn find_device_by_class(class_code: u8, subclass: u8) -> Option<PciDevice> {
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                if let Some(pci_device) = read_device(bus, device, function) {
                    if pci_device.class_code == class_code && pci_device.subclass == subclass {
                        return Some(pci_device);
                    }
                }
            }
        }
    }
    None
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
