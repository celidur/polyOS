#![allow(unused)]

use alloc::{sync::Arc, vec::Vec};
use core::result::Result;
use lazy_static::lazy_static;
use spin::Mutex;

pub trait BlockDevice: Send + Sync {
    /// Read `count` sectors from `lba` into `buf`.
    /// Return the number of sectors actually read or an error.
    fn read_sectors(
        &mut self,
        lba: u64,
        count: usize,
        buf: &mut [u8],
    ) -> Result<usize, BlockDeviceError>;

    /// Write `count` sectors from `buf` into `lba`.
    fn write_sectors(
        &mut self,
        lba: u64,
        count: usize,
        buf: &[u8],
    ) -> Result<usize, BlockDeviceError>;

    fn sector_size(&self) -> usize {
        512
    }
}

#[derive(Debug)]
pub enum BlockDeviceError {
    IoError,
    OutOfRange,
    InvalidArgument,
    NoSpace,
    NotFound,
}

lazy_static! {
    static ref BLOCK_DEVICES: Mutex<Vec<Arc<Mutex<dyn BlockDevice>>>> = Mutex::new(Vec::new());
}

pub fn register_block_device(dev: Arc<Mutex<dyn BlockDevice>>) -> usize {
    let mut devices = BLOCK_DEVICES.lock();
    devices.push(dev);
    devices.len() - 1
}

pub fn get_block_device(id: usize) -> Option<Arc<Mutex<dyn BlockDevice + 'static>>> {
    let devices = BLOCK_DEVICES.lock();
    devices.get(id).cloned()
}

// Example block device (e.g., a dummy RAM disk or real hardware)
pub struct MockBlockDevice {
    // In a real driver, store device state (ports, memory, etc.)
    // For simplicity, let's store an in-memory buffer as a "disk"
    storage: &'static mut [u8],
    sector_size: usize,
}

impl MockBlockDevice {
    pub fn new(storage: &'static mut [u8], sector_size: usize) -> Self {
        Self {
            storage,
            sector_size,
        }
    }
}

impl BlockDevice for MockBlockDevice {
    fn read_sectors(
        &mut self,
        lba: u64,
        count: usize,
        buf: &mut [u8],
    ) -> Result<usize, BlockDeviceError> {
        let start = (lba as usize) * self.sector_size;
        let end = start + count * self.sector_size;
        if end > self.storage.len() || buf.len() < (end - start) {
            return Err(BlockDeviceError::IoError);
        }
        buf[..(end - start)].copy_from_slice(&self.storage[start..end]);
        Ok(count)
    }

    fn write_sectors(
        &mut self,
        lba: u64,
        count: usize,
        buf: &[u8],
    ) -> Result<usize, BlockDeviceError> {
        let start = (lba as usize) * self.sector_size;
        let end = start + count * self.sector_size;
        if end > self.storage.len() || buf.len() < (end - start) {
            return Err(BlockDeviceError::IoError);
        }
        self.storage[start..end].copy_from_slice(&buf[..(end - start)]);
        Ok(count)
    }
}
