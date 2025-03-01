use core::{
    alloc::{Allocator, Layout},
    ffi::c_void,
    ptr::NonNull,
};
use lazy_static::lazy_static;

use alloc::{alloc::Global, vec::Vec};
use spin::Mutex;

use crate::serial_print;

const PAGE_SIZE: usize = 4096;

pub struct AllocationHeader {
    ptr: u32,
    size: usize,
    alignment: usize,
}

lazy_static! {
    pub static ref MEMORIES: Mutex<Vec<AllocationHeader>> = Mutex::new(Vec::new());
}

#[unsafe(no_mangle)]
pub extern "C" fn kmalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }

    let layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()).ok() {
        Some(layout) => layout,
        None => return core::ptr::null_mut(),
    };
    let alloc = Global;
    let ptr = alloc.allocate(layout);
    match ptr {
        Ok(ptr) => {
            let ptr = ptr.as_ptr() as *mut u8;

            MEMORIES.lock().push(AllocationHeader {
                ptr: ptr as u32,
                size,
                alignment: core::mem::align_of::<u8>(),
            });

            ptr as *mut core::ffi::c_void
        }
        Err(_) => core::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kzalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let layout = match Layout::from_size_align(size, core::mem::align_of::<u8>()).ok() {
        Some(layout) => layout,
        None => return core::ptr::null_mut(),
    };
    let ptr = Global.allocate_zeroed(layout);
    match ptr {
        Ok(ptr) => {
            let ptr = ptr.as_ptr() as *mut u8;

            MEMORIES.lock().push(AllocationHeader {
                ptr: ptr as u32,
                size,
                alignment: core::mem::align_of::<u8>(),
            });

            ptr as *mut core::ffi::c_void
        }
        Err(_) => core::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kpalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let layout = match Layout::from_size_align(size, PAGE_SIZE) {
        Ok(layout) => layout,
        Err(_) => return core::ptr::null_mut(),
    };

    let ptr = Global.allocate_zeroed(layout);
    match ptr {
        Ok(ptr) => {
            let raw_ptr = ptr.as_ptr() as *mut u8;

            MEMORIES.lock().push(AllocationHeader {
                ptr: raw_ptr as u32,
                size,
                alignment: PAGE_SIZE,
            });

            raw_ptr as *mut core::ffi::c_void
        }
        Err(_) => core::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kfree(ptr: *mut core::ffi::c_void) {
    if ptr.is_null() {
        return;
    }

    let mut index = None;
    for (i, header) in MEMORIES.lock().iter().enumerate() {
        if core::ptr::eq(header.ptr as *const c_void, ptr) {
            index = Some(i);
            break;
        }
    }

    if let Some(index) = index {
        let header = MEMORIES.lock().remove(index);
        if let Ok(layout) = Layout::from_size_align(header.size, header.alignment) {
            let raw_ptr = header.ptr as *mut u8;
            unsafe {
                Global.deallocate(NonNull::new_unchecked(raw_ptr), layout);
            }
            return;
        }
    }
    serial_print!("Failed to free memory at {:?}", ptr);
}
