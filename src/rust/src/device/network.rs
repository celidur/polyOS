#![allow(dead_code)]

use alloc::vec::Vec;

use crate::{
    constant::{irq_to_vector, PIC_MASTER_DATA_PORT, PIC_SLAVE_DATA_PORT, PIC_SLAVE_IRQ_MASK},
    device::{
        io::{inb as port_inb, inw as port_inw, outb, outl, outw},
        pci::{find_device, PciBar, PciDevice},
        DeviceDriver, DeviceProbeStage, ManagedDevice,
    },
    interrupts::{InterruptDevice, InterruptSource},
    memory::Page,
    net::{self, InterfaceId, NetworkDevice, NetworkError},
};

const RTL8139_VENDOR_ID: u16 = 0x10EC;
const RTL8139_DEVICE_ID: u16 = 0x8139;

const REG_IDR0: u16 = 0x00;
const REG_TSD0: u16 = 0x10;
const REG_TSAD0: u16 = 0x20;
const REG_RBSTART: u16 = 0x30;
const REG_COMMAND: u16 = 0x37;
const REG_CAPR: u16 = 0x38;
const REG_IMR: u16 = 0x3C;
const REG_ISR: u16 = 0x3E;
const REG_TCR: u16 = 0x40;
const REG_RCR: u16 = 0x44;
const REG_MPC: u16 = 0x4C;
const REG_CONFIG_9346: u16 = 0x50;
const REG_CONFIG1: u16 = 0x52;

const COMMAND_RX_BUFFER_EMPTY: u8 = 1 << 0;
const COMMAND_TX_ENABLE: u8 = 1 << 2;
const COMMAND_RX_ENABLE: u8 = 1 << 3;
const COMMAND_RESET: u8 = 1 << 4;

const ISR_RX_OK: u16 = 1 << 0;
const ISR_RX_ERR: u16 = 1 << 1;
const ISR_TX_OK: u16 = 1 << 2;
const ISR_TX_ERR: u16 = 1 << 3;
const ISR_RX_OVERFLOW: u16 = 1 << 4;
const ISR_RX_FIFO_OVERFLOW: u16 = 1 << 6;
const ISR_SYSTEM_ERR: u16 = 1 << 15;
const IMR_DEFAULT: u16 =
    ISR_RX_OK | ISR_RX_ERR | ISR_TX_OK | ISR_TX_ERR | ISR_RX_OVERFLOW | ISR_RX_FIFO_OVERFLOW
        | ISR_SYSTEM_ERR;

const RX_STATUS_OK: u16 = 1 << 0;

const RCR_ACCEPT_ALL: u32 = 1 << 0;
const RCR_ACCEPT_PHYSICAL: u32 = 1 << 1;
const RCR_ACCEPT_MULTICAST: u32 = 1 << 2;
const RCR_ACCEPT_BROADCAST: u32 = 1 << 3;
const RCR_WRAP: u32 = 1 << 7;
const RCR_MAX_DMA_UNLIMITED: u32 = 7 << 8;

const TCR_MAX_DMA_UNLIMITED: u32 = 7 << 8;
const TCR_INTERFRAME_GAP: u32 = 3 << 24;

const RX_RING_SIZE: usize = 8192;
const RX_BUFFER_SIZE: usize = RX_RING_SIZE + 16 + 1500;
const TX_BUFFER_COUNT: usize = 4;
const TX_BUFFER_SIZE: usize = 2048;
const ETHERNET_MIN_FRAME_SIZE: usize = 60;
const ETHERNET_MAX_FRAME_SIZE: usize = 1514;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rtl8139Error {
    DeviceNotFound,
    MissingIoBar,
    OutOfMemory,
    ResetTimeout,
    TxPacketTooLarge,
}

pub struct Rtl8139 {
    pci: PciDevice,
    io_base: u16,
    mac: [u8; 6],
    interface_id: InterfaceId,
    rx_buffer: Page<u8>,
    rx_offset: usize,
    tx_buffers: Vec<Page<u8>>,
    tx_current: usize,
    rx_errors: u64,
}

unsafe impl Send for Rtl8139 {}

pub struct Rtl8139Driver {
    device: ManagedDevice<Rtl8139>,
}

impl Rtl8139Driver {
    pub const fn new() -> Self {
        Self {
            device: ManagedDevice::new(),
        }
    }
}

pub static RTL8139_DRIVER: Rtl8139Driver = Rtl8139Driver::new();

impl DeviceDriver for Rtl8139Driver {
    fn name(&self) -> &'static str {
        "rtl8139"
    }

    fn stage(&self) -> DeviceProbeStage {
        DeviceProbeStage::Normal
    }

    fn probe(&self) {
        match Rtl8139::new() {
            Ok(mut device) => {
                let irq_line = device.pci.interrupt_line();
                let pci_bus = device.pci.bus;
                let pci_device = device.pci.device;
                let pci_function = device.pci.function;
                let io_base = device.io_base;

                let interface_id = net::register_interface(
                    "rtl8139",
                    0,
                    device.mac,
                    &RTL8139_DRIVER,
                );
                device.interface_id = interface_id;
                RTL8139_DRIVER
                    .device
                    .probe(device)
                    .expect("rtl8139 device already probed");

                if let Some(irq_vector) = irq_to_vector(irq_line) {
                    InterruptSource::new(irq_vector)
                        .register_device(&RTL8139_DRIVER);
                    enable_irq_line(irq_line);
                } else {
                    serial_println!(
                        "rtl8139: cannot register interrupt handler for PCI {}:{}:{}: invalid IRQ line {}",
                        pci_bus, pci_device, pci_function,
                        irq_line
                    );
                }

                serial_println!(
                    "rtl8139: initialized PCI {}:{}:{} io=0x{:x} irq={} net{}",
                    pci_bus,
                    pci_device,
                    pci_function,
                    io_base,
                    irq_line,
                    interface_id,
                );
            }
            Err(Rtl8139Error::DeviceNotFound) => {
                serial_println!("rtl8139: no QEMU RTL8139 device found");
            }
            Err(error) => {
                serial_println!("rtl8139: init failed: {:?}", error);
            }
        }
    }

    fn remove(&self) {
        let _ = RTL8139_DRIVER.device.remove();
    }
}

crate::register_device_driver!(RTL8139_DRIVER_REG, RTL8139_DRIVER);

impl NetworkDevice for Rtl8139Driver {
    fn read(&self) -> Option<Vec<u8>> {
        self.device.with_mut(|device| device.read_packet()).flatten()
    }

    fn write(&self, frame: &[u8]) -> Result<(), NetworkError> {
        self.device
            .with_mut(|device| device.send(frame))
            .ok_or(NetworkError::NoInterface)?
            .map_err(|_| NetworkError::DeviceError)
    }
}

impl InterruptDevice for Rtl8139Driver {
    fn interrupt(&self) {
        let Some(interface_id) = self.device.with(|device| device.interface_id) else {
            return;
        };

        while let Some(packet) = self.read() {
            if let Some(response) = net::receive(interface_id, packet.as_slice()) {
                match self.write(response.as_slice()) {
                    Ok(()) => net::notify_tx(interface_id),
                    Err(error) => serial_println!("rtl8139: tx response failed: {:?}", error),
                }
            }
        }
    }
}

impl Rtl8139 {
    fn new() -> Result<Self, Rtl8139Error> {
        let pci = find_device(RTL8139_VENDOR_ID, RTL8139_DEVICE_ID)
            .ok_or(Rtl8139Error::DeviceNotFound)?;
        let io_base = match pci.bar(0) {
            Some(PciBar::Io(base)) => base,
            _ => return Err(Rtl8139Error::MissingIoBar),
        };

        pci.enable_io_space();
        pci.enable_bus_mastering();

        let rx_buffer = Page::new(RX_BUFFER_SIZE).ok_or(Rtl8139Error::OutOfMemory)?;
        let mut tx_buffers = Vec::new();
        for _ in 0..TX_BUFFER_COUNT {
            tx_buffers.push(Page::new(TX_BUFFER_SIZE).ok_or(Rtl8139Error::OutOfMemory)?);
        }

        let mut device = Self {
            pci,
            io_base,
            mac: [0; 6],
            interface_id: 0,
            rx_buffer,
            rx_offset: 0,
            tx_buffers,
            tx_current: 0,
            rx_errors: 0,
        };

        device.reset()?;
        device.mac = device.read_mac();
        device.configure();

        Ok(device)
    }

    fn reset(&self) -> Result<(), Rtl8139Error> {
        self.outb(REG_CONFIG_9346, 0xC0);
        self.outb(REG_CONFIG1, 0x00);
        self.outb(REG_CONFIG_9346, 0x00);

        self.outb(REG_COMMAND, COMMAND_RESET);
        for _ in 0..100000 {
            if self.inb(REG_COMMAND) & COMMAND_RESET == 0 {
                return Ok(());
            }
        }

        Err(Rtl8139Error::ResetTimeout)
    }

    fn configure(&self) {
        self.outl(REG_RBSTART, self.rx_buffer.as_ptr() as u32);
        self.outw(REG_IMR, IMR_DEFAULT);
        self.outw(REG_ISR, u16::MAX);
        self.outl(REG_MPC, 0);
        self.outl(
            REG_RCR,
            RCR_ACCEPT_ALL
                | RCR_ACCEPT_PHYSICAL
                | RCR_ACCEPT_MULTICAST
                | RCR_ACCEPT_BROADCAST
                | RCR_WRAP
                | RCR_MAX_DMA_UNLIMITED,
        );
        self.outl(REG_TCR, TCR_INTERFRAME_GAP | TCR_MAX_DMA_UNLIMITED);
        self.outb(REG_COMMAND, COMMAND_TX_ENABLE | COMMAND_RX_ENABLE);
    }

    fn read_mac(&self) -> [u8; 6] {
        let mut mac = [0; 6];
        for (i, byte) in mac.iter_mut().enumerate() {
            *byte = self.inb(REG_IDR0 + i as u16);
        }
        mac
    }

    fn send(&mut self, frame: &[u8]) -> Result<(), Rtl8139Error> {
        if frame.len() > ETHERNET_MAX_FRAME_SIZE {
            return Err(Rtl8139Error::TxPacketTooLarge);
        }

        let tx_index = self.tx_current;
        let tx_len = frame.len().max(ETHERNET_MIN_FRAME_SIZE);
        {
            let tx_buffer = self.tx_buffers[tx_index].as_mut_slice();
            tx_buffer[..tx_len].fill(0);
            tx_buffer[..frame.len()].copy_from_slice(frame);
        }

        self.outl(
            REG_TSAD0 + tx_index as u16 * 4,
            self.tx_buffers[tx_index].as_ptr() as u32,
        );
        self.outl(REG_TSD0 + tx_index as u16 * 4, tx_len as u32);

        self.tx_current = (self.tx_current + 1) % TX_BUFFER_COUNT;
        Ok(())
    }

    fn read_packet(&mut self) -> Option<Vec<u8>> {
        if self.inb(REG_COMMAND) & COMMAND_RX_BUFFER_EMPTY != 0 {
            let status = self.inw(REG_ISR);
            if status != 0 {
                self.outw(
                    REG_ISR,
                    status
                        & (ISR_RX_OK
                            | ISR_RX_ERR
                            | ISR_TX_OK
                            | ISR_TX_ERR
                            | ISR_RX_OVERFLOW
                            | ISR_RX_FIFO_OVERFLOW
                            | ISR_SYSTEM_ERR),
                );
            }
            return None;
        }

        let packet = match self.read_packet_inner() {
            Ok(packet) => Some(packet),
            Err(()) => {
                self.rx_errors += 1;
                if self.rx_errors <= 4 {
                    serial_println!("rtl8139: rx error at offset {}", self.rx_offset);
                }
                self.outw(
                    REG_ISR,
                    ISR_RX_ERR | ISR_RX_OVERFLOW | ISR_RX_FIFO_OVERFLOW | ISR_SYSTEM_ERR,
                );
                None
            }
        };

        let status = self.inw(REG_ISR);
        if status != 0 {
            self.outw(
                REG_ISR,
                status
                    & (ISR_RX_OK
                        | ISR_RX_ERR
                        | ISR_TX_OK
                        | ISR_TX_ERR
                        | ISR_RX_OVERFLOW
                        | ISR_RX_FIFO_OVERFLOW
                        | ISR_SYSTEM_ERR),
            );
        }

        packet
    }

    fn read_packet_inner(&mut self) -> Result<Vec<u8>, ()> {
        let header_offset = self.rx_offset;
        let status = self.rx_read_u16(header_offset);
        let size = self.rx_read_u16(header_offset + 2) as usize;

        if status & RX_STATUS_OK == 0 || size < 4 || size > ETHERNET_MAX_FRAME_SIZE + 4 {
            return Err(());
        }

        let packet_len = size - 4;
        let packet = self.copy_rx_packet(header_offset + 4, packet_len);
        self.advance_rx(size + 4);
        Ok(packet)
    }

    fn copy_rx_packet(&self, packet_offset: usize, packet_len: usize) -> Vec<u8> {
        let mut packet = Vec::with_capacity(packet_len);
        for i in 0..packet_len {
            packet.push(self.rx_read_u8(packet_offset + i));
        }
        packet
    }

    fn advance_rx(&mut self, bytes: usize) {
        self.rx_offset = (self.rx_offset + bytes + 3) & !3;
        if self.rx_offset >= RX_RING_SIZE {
            self.rx_offset -= RX_RING_SIZE;
        }

        self.outw(REG_CAPR, self.rx_offset.wrapping_sub(16) as u16);
    }

    fn rx_read_u8(&self, offset: usize) -> u8 {
        let offset = if offset < RX_BUFFER_SIZE {
            offset
        } else {
            offset % RX_RING_SIZE
        };
        self.rx_buffer.as_slice()[offset]
    }

    fn rx_read_u16(&self, offset: usize) -> u16 {
        self.rx_read_u8(offset) as u16 | ((self.rx_read_u8(offset + 1) as u16) << 8)
    }

    fn inb(&self, offset: u16) -> u8 {
        unsafe { port_inb(self.io_base + offset) }
    }

    fn inw(&self, offset: u16) -> u16 {
        unsafe { port_inw(self.io_base + offset) }
    }

    fn outb(&self, offset: u16, value: u8) {
        unsafe { outb(self.io_base + offset, value) };
    }

    fn outw(&self, offset: u16, value: u16) {
        unsafe { outw(self.io_base + offset, value) };
    }

    fn outl(&self, offset: u16, value: u32) {
        unsafe { outl(self.io_base + offset, value) };
    }
}

fn enable_irq_line(irq_line: u8) {
    if irq_line >= 16 {
        return;
    }

    let mut master_mask = unsafe { port_inb(PIC_MASTER_DATA_PORT) };
    let mut slave_mask = unsafe { port_inb(PIC_SLAVE_DATA_PORT) };

    if irq_line < 8 {
        master_mask &= !1u8.wrapping_shl(irq_line as u32);
        unsafe { outb(PIC_MASTER_DATA_PORT, master_mask) };
        return;
    }

    master_mask &= !PIC_SLAVE_IRQ_MASK;
    slave_mask &= !1u8.wrapping_shl((irq_line - 8) as u32);

    unsafe {
        outb(PIC_MASTER_DATA_PORT, master_mask);
        outb(PIC_SLAVE_DATA_PORT, slave_mask);
    }
}
