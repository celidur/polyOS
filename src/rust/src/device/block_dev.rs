#![allow(unused)]

use alloc::vec::Vec;
use core::{fmt::Debug, result::Result};
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

pub trait BlockDevice: Send + Sync + Debug {
    /// Read `count` sectors from `lba` into `buf`.
    /// Return the number of sectors actually read or an error.
    fn read_sectors(
        &self,
        lba: u64,
        count: usize,
        buf: &mut [u8],
    ) -> Result<usize, BlockDeviceError>;

    /// Write `count` sectors from `buf` into `lba`.
    fn write_sectors(
        &self,
        lba: u64,
        count: usize,
        buf: &[u8],
    ) -> Result<usize, BlockDeviceError>;

    fn sector_size(&self) -> usize {
        512
    }

    fn sync(&self) -> Result<(), BlockDeviceError> {
        Ok(())
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

impl fatfs::IoError for BlockDeviceError {
    fn is_interrupted(&self) -> bool {
        false // none of your variants indicate an interrupt
    }

    fn new_unexpected_eof_error() -> Self {
        BlockDeviceError::IoError
    }

    fn new_write_zero_error() -> Self {
        BlockDeviceError::IoError
    }
}

lazy_static! {
    static ref BLOCK_DEVICES: RwLock<Vec<&'static dyn BlockDevice>> = RwLock::new(Vec::new());
}

pub fn register_block_device(device: &'static dyn BlockDevice) -> usize {
    let mut devices = BLOCK_DEVICES.write();
    devices.push(device);
    devices.len() - 1
}

fn block_device(id: usize) -> Option<&'static dyn BlockDevice> {
    BLOCK_DEVICES.read().get(id).copied()
}

pub fn read_sectors(
    id: usize,
    lba: u64,
    count: usize,
    buf: &mut [u8],
) -> Result<usize, BlockDeviceError> {
    block_device(id)
        .ok_or(BlockDeviceError::NotFound)?
        .read_sectors(lba, count, buf)
}

pub fn write_sectors(
    id: usize,
    lba: u64,
    count: usize,
    buf: &[u8],
) -> Result<usize, BlockDeviceError> {
    block_device(id)
        .ok_or(BlockDeviceError::NotFound)?
        .write_sectors(lba, count, buf)
}

pub fn sync_all() {
    for device in BLOCK_DEVICES.read().iter().copied() {
        let _ = device.sync();
    }
}
