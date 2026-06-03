use alloc::string::{String, ToString};

use crate::{
    memory::PageDirectory,
    schedule::task::{Task, copy_string_from_task, copy_string_to_task},
};

pub fn read_c_string(task: &Task, ptr: u32, max_len: usize) -> Option<String> {
    let mut buffer = vec![0_u8; max_len.max(1)];
    copy_from_user(
        &task.process.page_directory,
        ptr,
        buffer.as_mut_ptr(),
        buffer.len() as u32,
    )
    .ok()?;
    let last = buffer.len() - 1;
    buffer[last] = 0;

    let value = unsafe { core::ffi::CStr::from_ptr(buffer.as_ptr() as *const i8) }
        .to_str()
        .ok()?;
    Some(value.to_string())
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
    copy_string_from_task(directory, user_ptr, kernel_ptr as u32, size)
}

pub fn copy_to_user(
    directory: &PageDirectory,
    user_ptr: u32,
    kernel_ptr: *const u8,
    size: u32,
) -> Result<(), ()> {
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
