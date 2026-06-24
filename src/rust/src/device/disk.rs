#![allow(unused)]

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use bytemuck::cast_slice;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

use super::{
    block_dev::{BlockDevice, BlockDeviceError, register_block_device},
    driver::{DeviceDriver, DeviceProbeStage},
    io::{inb, inw, outb, outw},
    managed::ManagedDevice,
};

const ATA_REG_DATA: u16 = 0x0;
const ATA_REG_SECTOR_COUNT: u16 = 0x2;
const ATA_REG_LBA_LOW: u16 = 0x3;
const ATA_REG_LBA_MID: u16 = 0x4;
const ATA_REG_LBA_HIGH: u16 = 0x5;
const ATA_REG_DRIVE: u16 = 0x6;
const ATA_REG_STATUS_COMMAND: u16 = 0x7;

const ATA_STATUS_ERR: u8 = 0x01;
const ATA_STATUS_DRQ: u8 = 0x08;
const ATA_STATUS_DF: u8 = 0x20;
const ATA_STATUS_BSY: u8 = 0x80;

const ATA_CMD_READ_SECTORS: u8 = 0x20;
const ATA_CMD_WRITE_SECTORS: u8 = 0x30;
const ATA_DRIVE_LBA_MASTER: u8 = 0xE0;
const ATA_WAIT_TIMEOUT: usize = 1_000_000;

pub enum DiskError {
    Timeout,
    IoError,
}

#[derive(Debug)]
pub struct CacheEntry {
    pub data: [u16; 256],
    pub dirty: bool,
    pub nb_use: u64,
}

#[derive(Debug, Default)]
pub struct Disk {
    base: u16,
    cache: BTreeMap<u64, Arc<Mutex<CacheEntry>>>,
    cache_size: usize,
}

impl Disk {
    pub fn new(base: u16) -> Self {
        Self {
            base,
            cache: BTreeMap::new(),
            cache_size: 128,
        }
    }

    fn status(&self) -> u8 {
        unsafe { inb(self.base + ATA_REG_STATUS_COMMAND) }
    }

    fn wait_ready(&self) -> Result<(), DiskError> {
        let mut timeout = ATA_WAIT_TIMEOUT;
        while timeout > 0 {
            if self.status() & ATA_STATUS_BSY == 0 {
                return Ok(());
            }
            timeout -= 1;
        }

        Err(DiskError::Timeout)
    }

    fn wait_data_request(&self) -> Result<(), DiskError> {
        let mut timeout = ATA_WAIT_TIMEOUT;
        while timeout > 0 {
            let status = self.status();
            if status & (ATA_STATUS_ERR | ATA_STATUS_DF) != 0 {
                return Err(DiskError::IoError);
            }
            if status & ATA_STATUS_BSY == 0 && status & ATA_STATUS_DRQ != 0 {
                return Ok(());
            }
            timeout -= 1;
        }

        Err(DiskError::Timeout)
    }

    fn select_lba_sector(&self, lba: u64) -> Result<(), DiskError> {
        self.wait_ready()?;
        unsafe {
            outb(
                self.base + ATA_REG_DRIVE,
                ATA_DRIVE_LBA_MASTER | ((lba >> 24) as u8 & 0x0f),
            );
            outb(self.base + ATA_REG_SECTOR_COUNT, 1);
            outb(self.base + ATA_REG_LBA_LOW, lba as u8);
            outb(self.base + ATA_REG_LBA_MID, (lba >> 8) as u8);
            outb(self.base + ATA_REG_LBA_HIGH, (lba >> 16) as u8);
        }
        Ok(())
    }

    fn evict(&mut self) -> Result<(), DiskError> {
        if self.cache.len() >= self.cache_size {
            if let Some(&key) = self
                .cache
                .iter()
                .filter(|&(_, entry)| !entry.lock().dirty)
                .min_by_key(|&(_, entry)| entry.lock().nb_use)
                .map(|(k, _)| k)
            {
                self.cache.remove(&key);
            } else if let Some(&key) = self
                .cache
                .iter()
                .min_by_key(|&(_, entry)| entry.lock().nb_use)
                .map(|(k, _)| k)
            {
                self.write_cache(key)?;
                self.cache.remove(&key);
            }
        }

        Ok(())
    }

    fn write_cache(&mut self, lba: u64) -> Result<(), DiskError> {
        if let Some(entry) = self.cache.get(&lba).cloned() {
            let mut entry = entry.lock();
            if entry.dirty {
                self.select_lba_sector(lba)?;
                unsafe { outb(self.base + ATA_REG_STATUS_COMMAND, ATA_CMD_WRITE_SECTORS) };
                self.wait_data_request()?;

                for j in 0..256 {
                    unsafe { outw(self.base + ATA_REG_DATA, entry.data[j]) };
                }

                self.wait_ready()?;
                entry.dirty = false;
            }

            entry.nb_use = 0;
        }
        Ok(())
    }

    fn read_cache(&mut self, lba: u64) -> Result<Arc<Mutex<CacheEntry>>, DiskError> {
        if let Some(entry) = self.cache.get_mut(&lba) {
            entry.lock().nb_use += 1;
            return Ok(entry.clone());
        }

        let mut buf: [u16; 256] = [0; 256];

        self.select_lba_sector(lba)?;
        unsafe { outb(self.base + ATA_REG_STATUS_COMMAND, ATA_CMD_READ_SECTORS) };
        self.wait_data_request()?;

        for e in buf.iter_mut() {
            *e = unsafe { inw(self.base + ATA_REG_DATA) };
        }
        self.wait_ready()?;

        let entry = Arc::new(Mutex::new(CacheEntry {
            data: buf,
            dirty: false,
            nb_use: 0,
        }));
        self.evict()?;
        self.cache.insert(lba, entry.clone());
        Ok(entry)
    }

    pub fn read_sectors_internal(
        &mut self,
        lba: u64,
        count: usize,
        buf: &mut [u8],
    ) -> Result<(), DiskError> {
        for i in 0..count {
            let entry = self.read_cache(lba + i as u64)?;
            let entry = entry.lock();
            buf[i * 512..(i + 1) * 512].copy_from_slice(cast_slice(&entry.data));
        }
        Ok(())
    }

    pub fn write_sectors_internal(
        &mut self,
        lba: u64,
        count: usize,
        buf: &[u8],
    ) -> Result<(), DiskError> {
        for i in 0..count {
            let entry = self.read_cache(lba + i as u64)?;
            let mut entry = entry.lock();
            entry
                .data
                .copy_from_slice(cast_slice(&buf[i * 512..(i + 1) * 512]));
            entry.dirty = true;
        }
        Ok(())
    }

    pub fn sync(&mut self) -> Result<(), DiskError> {
        let keys = self.cache.keys().cloned().collect::<Vec<_>>();
        for &lba in keys.iter() {
            self.write_cache(lba)?;
        }
        Ok(())
    }
}

pub struct DiskDriver {
    device: ManagedDevice<Disk>,
    block_device_id: AtomicUsize,
}

impl core::fmt::Debug for DiskDriver {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DiskDriver")
            .field("block_device_id", &self.block_device_id())
            .finish()
    }
}

impl DiskDriver {
    pub const fn new() -> Self {
        Self {
            device: ManagedDevice::new(),
            block_device_id: AtomicUsize::new(usize::MAX),
        }
    }

    pub fn block_device_id(&self) -> Option<usize> {
        let id = self.block_device_id.load(Ordering::Acquire);
        if id == usize::MAX { None } else { Some(id) }
    }
}

pub static DISK_DRIVER: DiskDriver = DiskDriver::new();

impl BlockDevice for DiskDriver {
    fn read_sectors(
        &self,
        lba: u64,
        count: usize,
        buf: &mut [u8],
    ) -> Result<usize, BlockDeviceError> {
        self.device
            .with_mut(|disk| {
                disk.read_sectors_internal(lba, count, buf)
                    .map_err(|_| BlockDeviceError::IoError)?;
                Ok(count)
            })
            .ok_or(BlockDeviceError::NotFound)?
    }

    fn write_sectors(&self, lba: u64, count: usize, buf: &[u8]) -> Result<usize, BlockDeviceError> {
        self.device
            .with_mut(|disk| {
                disk.write_sectors_internal(lba, count, buf)
                    .map_err(|_| BlockDeviceError::IoError)?;
                Ok(count)
            })
            .ok_or(BlockDeviceError::NotFound)?
    }

    fn sync(&self) -> Result<(), BlockDeviceError> {
        self.device
            .with_mut(|disk| disk.sync().map_err(|_| BlockDeviceError::IoError))
            .ok_or(BlockDeviceError::NotFound)?
    }
}

impl DeviceDriver for DiskDriver {
    fn name(&self) -> &'static str {
        "disk"
    }

    fn stage(&self) -> DeviceProbeStage {
        DeviceProbeStage::Early
    }

    fn probe(&self) {
        self.device
            .probe(Disk::new(0x1F0))
            .expect("disk device already probed");
        let id = register_block_device(&DISK_DRIVER);
        self.block_device_id.store(id, Ordering::Release);
    }

    fn remove(&self) {
        if let Some(mut disk) = self.device.remove() {
            let _ = disk.sync();
        }
        self.block_device_id.store(usize::MAX, Ordering::Release);
    }
}

crate::register_device_driver!(DISK_DRIVER_REG, DISK_DRIVER);
