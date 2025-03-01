use crate::device::block_dev::{BlockDevice, BlockDeviceError};
use alloc::sync::Arc;
use spin::Mutex;

use super::utils::fat_offset;

pub struct FatTable {
    pub first_fat_sector: u32,
    pub sectors_per_fat: u16,
    pub num_fats: u8,
}

impl FatTable {
    pub fn read_fat16_entry(
        &self,
        dev: Arc<Mutex<dyn BlockDevice>>,
        cluster: u16,
        bytes_per_sector: u16,
    ) -> Result<u16, BlockDeviceError> {
        let offset = fat_offset(cluster);
        let sec_size = bytes_per_sector as u32;
        let sector_index = offset / sec_size;
        let offset_in_sector = (offset % sec_size) as usize;

        let sector_lba = self.first_fat_sector + sector_index;
        let mut buf = [0u8; 512];
        dev.lock().read_sectors(sector_lba as u64, 1, &mut buf)?;
        let val = u16::from_le_bytes([buf[offset_in_sector], buf[offset_in_sector + 1]]);
        Ok(val)
    }

    pub fn write_fat16_entry(
        &self,
        dev: Arc<Mutex<dyn BlockDevice>>,
        cluster: u16,
        value: u16,
        bytes_per_sector: u16,
    ) -> Result<(), BlockDeviceError> {
        let offset = fat_offset(cluster);
        let sec_size = bytes_per_sector as u32;
        for i in 0..self.num_fats {
            let fat_start = self.first_fat_sector + (self.sectors_per_fat as u32 * i as u32);
            let sector_index = offset / sec_size;
            let offset_in_sector = (offset % sec_size) as usize;
            let sector_lba = fat_start + sector_index;
            let mut buf = [0u8; 512];
            dev.lock().read_sectors(sector_lba as u64, 1, &mut buf)?;
            let b = value.to_le_bytes();
            buf[offset_in_sector] = b[0];
            buf[offset_in_sector + 1] = b[1];
            dev.lock().write_sectors(sector_lba as u64, 1, &buf)?;
        }
        Ok(())
    }

    pub fn free_cluster_chain(
        &self,
        dev: Arc<Mutex<dyn BlockDevice>>,
        start: u16,
        bytes_per_sector: u16,
    ) -> Result<(), BlockDeviceError> {
        if start < 2 {
            return Ok(());
        }
        let mut current = start;
        loop {
            if !(2..0xFFF8).contains(&current) {
                break;
            }
            let next = self.read_fat16_entry(dev.clone(), current, bytes_per_sector)?;
            self.write_fat16_entry(dev.clone(), current, 0x0000, bytes_per_sector)?;
            current = next;
        }
        Ok(())
    }

    pub fn alloc_cluster(
        &self,
        dev: Arc<Mutex<dyn BlockDevice>>,
        bytes_per_sector: u16,
        _start: u16,
        total_clusters: u16,
    ) -> Result<u16, BlockDeviceError> {
        for i in 2..total_clusters {
            let c = i;
            let val = self.read_fat16_entry(dev.clone(), c, bytes_per_sector)?;
            if val == 0x0000 {
                self.write_fat16_entry(dev.clone(), c, 0xFFFF, bytes_per_sector)?;
                return Ok(c);
            }
        }
        Err(BlockDeviceError::NoSpace)
    }

    // pub fn get_end_of_chain(
    //     &self,
    //     dev: Arc<Mutex<dyn BlockDevice>>,
    //     cluster: u16,
    //     bytes_per_sector: u16,
    // ) -> Result<u16, BlockDeviceError> {
    //     let mut cur = cluster;
    //     loop {
    //         let next = self.read_fat16_entry(dev.clone(), cur, bytes_per_sector)?;
    //         if next < 2 || next >= 0xFFF8 {
    //             return Ok(cur);
    //         }
    //         cur = next;
    //     }
    // }

    pub fn extend_chain(
        &self,
        dev: Arc<Mutex<dyn BlockDevice>>,
        cluster: u16,
        newc: u16,
        bytes_per_sector: u16,
    ) -> Result<(), BlockDeviceError> {
        self.write_fat16_entry(dev.clone(), cluster, newc, bytes_per_sector)?;
        self.write_fat16_entry(dev, newc, 0xFFFF, bytes_per_sector)?;
        Ok(())
    }
}
