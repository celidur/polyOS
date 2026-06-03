use core::fmt::Write;

use alloc::boxed::Box;

use uart_16550::{ByteReceiveError, Config, Uart16550, backend::PioBackend};

use crate::{
    device::{DeviceDriver, DeviceProbeStage, ManagedDevice},
    fs::{FileHandle, FileMetadata, FileOps, FsError},
};

const DEFAULT_SERIAL_BASE: u16 = 0x3F8;

pub struct SerialPort {
    port: Uart16550<PioBackend>,
}

impl SerialPort {
    pub unsafe fn new(base: u16) -> Option<Self> {
        let mut port = unsafe { Uart16550::new_port(base) }.ok()?;
        port.init(Config::default()).ok()?;
        Some(Self { port })
    }

    pub fn send_bytes_exact(&mut self, bytes: &[u8]) {
        self.port.send_bytes_exact(bytes);
    }

    pub fn try_receive_byte(&mut self) -> Result<u8, ByteReceiveError> {
        self.port.try_receive_byte()
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.send_bytes_exact(s.as_bytes());
        Ok(())
    }
}

pub struct SerialDriver {
    device: ManagedDevice<SerialPort>,
}

impl SerialDriver {
    pub const fn new() -> Self {
        Self {
            device: ManagedDevice::new(),
        }
    }

    pub fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        self.device
            .with_mut(|serial_port: &mut SerialPort| serial_port.write_fmt(args))
            .expect("serial device not probed")
    }

    pub fn write(&self, bytes: &[u8]) {
        self.device
            .with_mut(|serial_port: &mut SerialPort| {
                serial_port.send_bytes_exact(bytes);
            })
            .expect("serial device not probed");
    }

    pub fn read(&self) -> Option<u8> {
        self.device
            .with_mut(|serial_port: &mut SerialPort| serial_port.try_receive_byte().ok())
            .flatten()
    }
}

pub static SERIAL_DRIVER: SerialDriver = SerialDriver::new();

struct SerialFile;

impl FileOps for SerialFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        let mut read = 0;

        while read < buf.len() {
            let Some(byte) = SERIAL_DRIVER.read().map(normalize_input) else {
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
        SERIAL_DRIVER.write(buf);
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
        let serial_port =
            unsafe { SerialPort::new(DEFAULT_SERIAL_BASE).expect("should be valid port") };

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
