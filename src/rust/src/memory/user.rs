use alloc::string::String;

use crate::{
    memory::PageDirectory,
    schedule::task::{Task, copy_string_from_task, copy_string_to_task},
};

pub fn read_c_string(task: &Task, ptr: u32, max_len: usize) -> Option<String> {
    read_process_string(&task.process, ptr, max_len)
}

pub fn read_process_string(
    process: &crate::schedule::process::Process,
    ptr: u32,
    max_len: usize,
) -> Option<String> {
    let mut bytes = alloc::vec::Vec::new();
    while bytes.len() < max_len {
        let address = ptr.checked_add(bytes.len() as u32)?;
        let count = (4096 - (address as usize & 4095)).min(max_len - bytes.len());
        let mut chunk = vec![0; count];
        copy_from_user(
            &process.page_directory,
            address,
            chunk.as_mut_ptr(),
            count as u32,
        )
        .ok()?;
        if let Some(nul) = chunk.iter().position(|&c| c == 0) {
            bytes.extend_from_slice(&chunk[..nul]);
            return String::from_utf8(bytes).ok();
        }
        bytes.extend_from_slice(&chunk);
    }
    None
}

pub fn read_u32(task: &Task, ptr: u32) -> Option<u32> {
    let mut value = 0_u32;
    copy_from_user(
        &task.process.page_directory,
        ptr,
        &mut value as *mut u32 as *mut u8,
        core::mem::size_of::<u32>() as u32,
    )
    .ok()?;
    Some(value)
}

pub fn copy_from_user(
    directory: &PageDirectory,
    user_ptr: u32,
    kernel_ptr: *mut u8,
    size: u32,
) -> Result<(), ()> {
    validate_directory_range(directory, user_ptr, size, false)?;
    copy_string_from_task(directory, user_ptr, kernel_ptr as u32, size)
}

pub fn copy_to_user(
    directory: &PageDirectory,
    user_ptr: u32,
    kernel_ptr: *const u8,
    size: u32,
) -> Result<(), ()> {
    validate_directory_range(directory, user_ptr, size, true)?;
    copy_string_to_task(directory, kernel_ptr as u32, user_ptr, size)
}

pub fn write_value<T>(directory: &PageDirectory, user_ptr: u32, value: &T) -> Result<(), ()> {
    copy_to_user(
        directory,
        user_ptr,
        value as *const T as *const u8,
        core::mem::size_of::<T>() as u32,
    )
}

/// Validate user pages before copying, resolving COW before a kernel write.
pub fn validate_range(
    process: &crate::schedule::process::Process,
    ptr: u32,
    len: u32,
    write: bool,
) -> Result<(), ()> {
    use crate::memory::{PRESENT, USER_ACCESS, WRITABLE};
    if len == 0 {
        return Ok(());
    }
    let end = ptr.checked_add(len - 1).ok_or(())?;
    if ptr == 0 {
        return Err(());
    }
    let mut page = ptr & !4095;
    loop {
        let mut entry = process.page_directory.get(page).map_err(|_| ())?;
        if entry & (PRESENT | USER_ACCESS) != PRESENT | USER_ACCESS {
            return Err(());
        }
        if write && entry & WRITABLE == 0 {
            if !process.handle_cow_fault(page).map_err(|_| ())? {
                return Err(());
            }
            entry = process.page_directory.get(page).map_err(|_| ())?;
            if entry & WRITABLE == 0 {
                return Err(());
            }
        }
        if page == end & !4095 {
            break;
        }
        page = page.checked_add(4096).ok_or(())?;
    }
    Ok(())
}

/// Device ioctls and syscall adapters share the same user-address validation.
pub fn validate_directory_range(
    directory: &PageDirectory,
    ptr: u32,
    len: u32,
    write: bool,
) -> Result<(), ()> {
    use crate::memory::page_directory::flags::COW;
    use crate::memory::{PRESENT, USER_ACCESS, WRITABLE};
    if len == 0 {
        return Ok(());
    }
    let end = ptr.checked_add(len - 1).ok_or(())?;
    if ptr == 0 {
        return Err(());
    }
    let mut page = ptr & !4095;
    loop {
        let entry = directory.get(page).map_err(|_| ())?;
        if entry & (PRESENT | USER_ACCESS) != (PRESENT | USER_ACCESS)
            || (write && entry & (WRITABLE | COW) == 0)
        {
            return Err(());
        }
        if page == end & !4095 {
            return Ok(());
        }
        page = page.checked_add(4096).ok_or(())?;
    }
}
