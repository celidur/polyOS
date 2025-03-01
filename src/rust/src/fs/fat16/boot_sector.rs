use core::mem::size_of;

use crate::device::block_dev::BlockDeviceError;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct BootSector {
    pub jump_boot: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    pub drive_number: u8,
    pub reserved1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type_label: [u8; 8],
}

impl BootSector {
    pub fn read_from_buffer(buf: &[u8]) -> Result<Self, BlockDeviceError> {
        if buf.len() < size_of::<BootSector>() {
            return Err(BlockDeviceError::InvalidArgument);
        }
        let boot_sector: Self = unsafe { core::ptr::read(buf.as_ptr() as *const _) };
        if u16::from_le_bytes([buf[510], buf[511]]) != 0xAA55 {
            return Err(BlockDeviceError::InvalidArgument);
        }
        Ok(boot_sector)
    }
}
