use core::arch::asm;

use crate::{
    constant::{PAGING_PAGE_SIZE, PAGING_PAGE_TABLE_SIZE},
    memory::page::Page,
};

#[allow(dead_code)]
pub mod flags {
    pub const PRESENT: u32 = 1 << 0;
    pub const WRITABLE: u32 = 1 << 1;
    pub const USER_ACCESS: u32 = 1 << 2; // ACCESS_FROM_ALL
    pub const WRITE_THROUGH: u32 = 1 << 3;
    pub const CACHE_DISABLED: u32 = 1 << 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingError {
    InvalidArg,
}

#[derive(Debug)]
pub struct PageDirectory {
    pub directory: Page,
    _entries: Page,
}

unsafe impl Send for PageDirectory {}
unsafe impl Sync for PageDirectory {}

impl PageDirectory {
    pub fn new_4gb(flags: u32) -> Option<Self> {
        let mut directory = Page::new(core::mem::size_of::<u32>() * PAGING_PAGE_TABLE_SIZE)?;
        let mut entries = Page::new(
            core::mem::size_of::<u32>() * PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_TABLE_SIZE,
        )?;

        let directory_raw = unsafe {
            core::slice::from_raw_parts_mut(
                directory.as_mut_ptr() as *mut u32,
                PAGING_PAGE_TABLE_SIZE,
            )
        };
        let entries_raw = unsafe {
            core::slice::from_raw_parts_mut(
                entries.as_mut_ptr() as *mut u32,
                PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_TABLE_SIZE,
            )
        };

        let mut offset = 0;
        for i in 0..PAGING_PAGE_TABLE_SIZE {
            let entry =
                entries_raw[i * PAGING_PAGE_TABLE_SIZE..(i + 1) * PAGING_PAGE_TABLE_SIZE].as_mut();
            for (b, e) in entry.iter_mut().enumerate().take(PAGING_PAGE_TABLE_SIZE) {
                *e = (offset + (b * PAGING_PAGE_SIZE) as u32) | flags;
            }
            offset += (PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_SIZE) as u32;
            directory_raw[i] = (entry.as_ptr() as u32 | flags) as u32;
        }

        Some(Self {
            _entries: entries,
            directory,
        })
    }

    pub fn switch(&self) {
        let directory = self.directory.as_ptr();

        unsafe {
            asm!(
                "mov cr3, eax",
                in("eax") directory as u32,
                options(nostack, preserves_flags)
            );
        }
    }

    fn is_aligned(address: u32) -> bool {
        address.is_multiple_of(PAGING_PAGE_SIZE as u32)
    }

    pub fn map(
        &self,
        virtual_address: u32,
        physical_address: u32,
        flags: u32,
    ) -> Result<(), PagingError> {
        if !Self::is_aligned(virtual_address) || !Self::is_aligned(physical_address) {
            return Err(PagingError::InvalidArg);
        }
        self.set(virtual_address, physical_address | flags)
    }

    pub fn set(&self, virtual_address: u32, value: u32) -> Result<(), PagingError> {
        if !Self::is_aligned(virtual_address) {
            return Err(PagingError::InvalidArg);
        }

        let (directory_index, table_index) = self.get_index(virtual_address)?;

        let entry = unsafe {
            let directory_raw = core::slice::from_raw_parts_mut(
                self.directory.as_ptr() as *mut u32,
                PAGING_PAGE_TABLE_SIZE,
            );
            &mut directory_raw[directory_index as usize]
        };

        let table = unsafe {
            let table_ptr = (*entry & 0xFFFFF000) as *mut u32;
            core::slice::from_raw_parts_mut(table_ptr, PAGING_PAGE_TABLE_SIZE)
        };

        table[table_index as usize] = value;
        let flags = self.get_highest_flag(table);
        *entry = (*entry & 0xFFFFF000) | flags;

        Ok(())
    }

    fn get_index(&self, virtual_addr: u32) -> Result<(u32, u32), PagingError> {
        if !Self::is_aligned(virtual_addr) {
            return Err(PagingError::InvalidArg);
        }

        let directory_index_out = virtual_addr / (PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) as u32;
        let table_index_out = virtual_addr % (PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) as u32
            / PAGING_PAGE_SIZE as u32;

        Ok((directory_index_out, table_index_out))
    }

    fn get_highest_flag(&self, table: &[u32]) -> u32 {
        let mut flags = 0;
        for &entry in table.iter() {
            flags |= entry & 7;
        }
        flags
    }

    pub fn map_range(
        &self,
        virtual_address: u32,
        physical_address: u32,
        count: u32,
        flags: u32,
    ) -> Result<(), PagingError> {
        for i in 0..count {
            self.map(
                virtual_address + i * PAGING_PAGE_SIZE as u32,
                physical_address + i * PAGING_PAGE_SIZE as u32,
                flags,
            )?;
        }
        Ok(())
    }

    pub fn map_to(
        &self,
        virtual_address: u32,
        physical_address: u32,
        physical_address_end: u32,
        flags: u32,
    ) -> Result<(), PagingError> {
        if physical_address_end < physical_address {
            return Err(PagingError::InvalidArg);
        }
        if !Self::is_aligned(physical_address_end) {
            return Err(PagingError::InvalidArg);
        }
        let count = (physical_address_end - physical_address) / PAGING_PAGE_SIZE as u32;
        self.map_range(virtual_address, physical_address, count, flags)
    }

    pub fn align_address(address: u32) -> u32 {
        if Self::is_aligned(address) {
            address
        } else {
            (address).saturating_add(PAGING_PAGE_SIZE as u32) & 0xFFFFF000
        }
    }

    pub fn align_address_down(address: u32) -> u32 {
        address & 0xFFFFF000
    }

    pub fn get(&self, virtual_address: u32) -> Result<u32, PagingError> {
        if !Self::is_aligned(virtual_address) {
            return Err(PagingError::InvalidArg);
        }

        let (directory_index, table_index) = self.get_index(virtual_address)?;

        let entry = unsafe {
            let directory_raw = core::slice::from_raw_parts_mut(
                self.directory.as_ptr() as *mut u32,
                PAGING_PAGE_TABLE_SIZE,
            );
            &directory_raw[directory_index as usize]
        };

        let table = unsafe {
            let table_ptr = (*entry & 0xFFFFF000) as *const u32;
            core::slice::from_raw_parts(table_ptr, PAGING_PAGE_TABLE_SIZE)
        };

        Ok(table[table_index as usize])
    }

    pub fn get_physical_address(&self, virtual_address: u32) -> Result<u32, PagingError> {
        let virt_addr_new = Self::align_address_down(virtual_address);
        let difference = virtual_address - virt_addr_new;
        Ok((self.get(virt_addr_new)? & 0xFFFFF000) + difference)
    }

    pub fn print_info(&self) {
        serial_println!("Paging info: ");
        let mut flag = 0;
        let mut start = 0xFFFFFFFF;
        let mut end = 0;
        for i in 0..PAGING_PAGE_TABLE_SIZE {
            let entry = unsafe { (self.directory.as_ptr() as *const u32).add(i) };
            let table = unsafe {
                let table_ptr = (*entry & 0xFFFFF000) as *const u32;
                core::slice::from_raw_parts(table_ptr, PAGING_PAGE_TABLE_SIZE)
            };
            for (b, f) in table.iter().enumerate().take(PAGING_PAGE_TABLE_SIZE) {
                let flag2 = f & 31;
                if flag2 != flag {
                    if start != 0xFFFFFFFF {
                        serial_print!("0x{:x} - 0x{:x}: ", start, end);
                        if flag & flags::PRESENT != 0 {
                            serial_print!("PRESENT ");
                        }
                        if flag & flags::WRITABLE != 0 {
                            serial_print!("WRITABLE ");
                        }
                        if flag & flags::USER_ACCESS != 0 {
                            serial_print!("USER_ACCESS ");
                        }
                        if flag & flags::WRITE_THROUGH != 0 {
                            serial_print!("WRITE_THROUGH ");
                        }
                        if flag & flags::CACHE_DISABLED != 0 {
                            serial_print!("CACHE_DISABLED ");
                        }
                        serial_println!();
                    }
                    start =
                        (i * PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) + (b * PAGING_PAGE_SIZE);
                    flag = flag2;
                }
                end = (i * PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE)
                    + (b * PAGING_PAGE_SIZE)
                    + 0xFFF;
            }
        }

        if start != 0xFFFFFFFF {
            serial_print!("0x{:x} - 0x{:x}: ", start, end);
            if flag & flags::PRESENT != 0 {
                serial_print!("PRESENT ");
            }
            if flag & flags::WRITABLE != 0 {
                serial_print!("WRITABLE ");
            }
            if flag & flags::USER_ACCESS != 0 {
                serial_print!("USER_ACCESS ");
            }
            if flag & flags::WRITE_THROUGH != 0 {
                serial_print!("WRITE_THROUGH ");
            }
            if flag & flags::CACHE_DISABLED != 0 {
                serial_print!("CACHE_DISABLED ");
            }
            serial_println!();
        }
    }

    pub fn map_page(
        &self,
        virtual_address: u32,
        page: &Page,
        flags: u32,
    ) -> Result<(), PagingError> {
        self.map_range(
            virtual_address,
            page.as_ptr() as u32,
            (page.len() / PAGING_PAGE_SIZE) as u32,
            flags,
        )
    }
}

pub fn enable_paging() {
    unsafe {
        asm!(
            "mov eax, cr0",
            "or eax, 0x80000000",
            "mov cr0, eax",
            options(nostack, preserves_flags)
        );
    }
}
