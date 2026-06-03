use alloc::boxed::Box;
use core::fmt::{self, Write};

use uart_16550::SerialPort;

use crate::{
    device::{DeviceDriver, DeviceProbeStage, ManagedDevice},
    fs::{FileHandle, FileMetadata, FileOps, FsError},
};

const DEFAULT_SERIAL_BASE: u16 = 0x3F8;

pub struct SerialDriver {
    device: ManagedDevice<SerialPort>,
}

impl SerialDriver {
    pub const fn new() -> Self {
        Self {
            device: ManagedDevice::new(),
        }
    }

    pub fn write(&self, args: fmt::Arguments) {
        self.device
            .with_mut(|serial_port| {
                serial_port
                    .write_fmt(args)
                    .expect("Printing to serial failed");
            })
            .expect("serial device not probed");
    }

    pub fn write_byte(&self, byte: u8) {
        self.device
            .with_mut(|serial_port| {
                serial_port.send(byte);
            })
            .expect("serial device not probed");
    }

    pub fn read_byte(&self) -> Option<u8> {
        self.device
            .with_mut(|serial_port| serial_port.try_receive().ok())
            .flatten()
    }
}

pub static SERIAL_DRIVER: SerialDriver = SerialDriver::new();

struct SerialFile;

impl FileOps for SerialFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        let mut read = 0;

        while read < buf.len() {
            let Some(byte) = SERIAL_DRIVER.read_byte().map(normalize_input) else {
                break;
            };

            buf[read] = byte;
            read += 1;

            if byte == 13 {
                break;
            }
        }

        Ok(read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        for byte in buf.iter().copied() {
            SERIAL_DRIVER.write_byte(byte);
        }

        Ok(buf.len())
    }

    fn seek(&mut self, _pos: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(FileMetadata {
            uid: 0,
            gid: 0,
            mode: 0o666,
            size: 0,
            is_dir: false,
        })
    }
}

impl DeviceDriver for SerialDriver {
    fn name(&self) -> &'static str {
        "serial"
    }

    fn stage(&self) -> DeviceProbeStage {
        DeviceProbeStage::Early
    }

    fn probe(&self) {
        let mut serial_port = unsafe { SerialPort::new(DEFAULT_SERIAL_BASE) };
        serial_port.init();

        SERIAL_DRIVER
            .device
            .probe(serial_port)
            .expect("serial device already probed");
    }

    fn remove(&self) {
        let _ = SERIAL_DRIVER.device.remove();
    }
}

pub fn open_serial() -> FileHandle {
    FileHandle::new(Box::new(SerialFile))
}

fn normalize_input(byte: u8) -> u8 {
    match byte {
        b'\n' => 13,
        0x7f => 0x08,
        byte => byte,
    }
}

crate::register_device_node!(SERIAL_DEVICE_NODE_REG, ["serial", "ttyS0"], open_serial);
crate::register_device_driver!(SERIAL_DRIVER_REG, SERIAL_DRIVER);
