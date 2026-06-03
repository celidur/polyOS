#![allow(unused)]

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use bytemuck::cast_slice;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

use super::{
    block_dev::{register_block_device, BlockDevice, BlockDeviceError},
    driver::{DeviceDriver, DeviceProbeStage},
    managed::ManagedDevice,
    io::{inb, inw, outb, outw},
};

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
        if let Some(entry) = self.cache.get_mut(&lba) {
            let mut entry = entry.lock();
            if entry.dirty {
                unsafe {
                    outb(self.base + 0x6, (lba >> 24) as u8 | 0xE0);
                    outb(self.base + 0x2, 1_u8);
                    outb(self.base + 0x3, lba as u8);
                    outb(self.base + 0x4, (lba >> 8) as u8);
                    outb(self.base + 0x5, (lba >> 16) as u8);
                    outb(self.base + 0x7, 0x30);
                }
                let mut timeout = 1000000;
                while timeout > 0 {
                    let status = unsafe { inb(self.base + 0x7) };
                    if status & 0x80 == 0 && status & 0x08 != 0 {
                        break;
                    }
                    timeout -= 1;
                }
                if timeout == 0 {
                    return Err(DiskError::Timeout);
                }

                for j in 0..256 {
                    unsafe { outw(self.base, entry.data[j]) };
                }

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

        unsafe {
            outb(self.base + 0x6, (lba >> 24) as u8 | 0xE0);
            outb(self.base + 0x2, 1_u8);
            outb(self.base + 0x3, lba as u8);
            outb(self.base + 0x4, (lba >> 8) as u8);
            outb(self.base + 0x5, (lba >> 16) as u8);
            outb(self.base + 0x7, 0x20);
        }

        let mut timeout = 1000000;
        while unsafe { inb(self.base + 0x7) } & 0x80 != 0 && timeout > 0 {
            timeout -= 1;
        }

        if timeout == 0 {
            return Err(DiskError::Timeout);
        }

        for e in buf.iter_mut() {
            *e = unsafe { inw(self.base) };
        }

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
        if id == usize::MAX {
            None
        } else {
            Some(id)
        }
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

    fn write_sectors(
        &self,
        lba: u64,
        count: usize,
        buf: &[u8],
    ) -> Result<usize, BlockDeviceError> {
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
