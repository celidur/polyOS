// use core::alloc::{GlobalAlloc, Layout};

use core::{alloc::{Allocator, Layout}, ptr::NonNull};

use alloc::alloc::Global;

use crate::serial_print;

const PAGE_SIZE: usize = 4096;

const MAGIC_NUMBER: u32 = 0x1CAFE1;

#[repr(C)]
struct AllocationHeader {
    size: usize,
    alignment: usize,
    magic: u32,
}

#[no_mangle]
pub extern "C" fn rust_kmalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let total_size = size + core::mem::size_of::<AllocationHeader>();

    let layout = match Layout::from_size_align(total_size, core::mem::align_of::<u8>()).ok() {
        Some(layout) => layout,
        None => return core::ptr::null_mut(),
    };
    let alloc = Global;
    let ptr = alloc.allocate(layout);
    match ptr {
        Ok(ptr) => {
            let ptr = ptr.as_ptr() as *mut u8;
            serial_print!("Allocated {:?} bytes at {:p}\n", total_size, ptr);
            let header = ptr as *mut AllocationHeader;
            unsafe {
                (*header).size = total_size;
                (*header).alignment = core::mem::align_of::<u8>();
                (*header).magic = MAGIC_NUMBER;
            }
            unsafe { ptr.add(core::mem::size_of::<AllocationHeader>()) as *mut core::ffi::c_void } 
        },
        Err(_) => core::ptr::null_mut(),
    }
}



#[no_mangle]
pub extern "C" fn rust_kzalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let total_size = size + core::mem::size_of::<AllocationHeader>();
    let layout = match Layout::from_size_align(total_size, core::mem::align_of::<u8>()).ok() {
        Some(layout) => layout,
        None => return core::ptr::null_mut(),
    };
    let ptr = Global.allocate_zeroed(layout);
    match ptr {
        Ok(ptr) => {
            let ptr = ptr.as_ptr() as *mut u8;
            serial_print!("Allocated {:?} bytes at {:p}\n", total_size, ptr);
            let header = ptr as *mut AllocationHeader;
            unsafe {
                (*header).size = total_size;
                (*header).alignment = core::mem::align_of::<usize>();
                (*header).magic = MAGIC_NUMBER;
            }
            unsafe { ptr.add(core::mem::size_of::<AllocationHeader>()) as *mut core::ffi::c_void } 
        },
        Err(_) => core::ptr::null_mut(),
    }
}



#[no_mangle]
pub extern "C" fn rust_kpalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let total_size = aligned_size + PAGE_SIZE;
    let layout = match Layout::from_size_align(total_size, PAGE_SIZE){
        Ok(layout) => layout,
        Err(_) => return core::ptr::null_mut(),
    };

    let ptr = Global.allocate_zeroed(layout);
    match ptr {
        Ok(ptr) => {
            let raw_ptr = ptr.as_ptr() as *mut u8;
            serial_print!("Allocated {:?} bytes at {:p}\n", total_size, raw_ptr);

            let aligned_ptr = (unsafe { raw_ptr.add(core::mem::size_of::<AllocationHeader>()) } as usize
                + (PAGE_SIZE - 1))
                & !(PAGE_SIZE - 1);
            let aligned_ptr = aligned_ptr as *mut u8;

            let header = unsafe { (aligned_ptr as *mut u8).sub(core::mem::size_of::<AllocationHeader>()) } as *mut AllocationHeader;
            unsafe {
                (*header).size = total_size;
                (*header).alignment = PAGE_SIZE;
                (*header).magic = MAGIC_NUMBER;
            }

            aligned_ptr as *mut core::ffi::c_void
        },
        Err(_) => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rust_kfree(ptr: *mut core::ffi::c_void) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header = (ptr as *mut u8).sub(core::mem::size_of::<AllocationHeader>()) as *mut AllocationHeader;
        
        let total_size = (*header).size;
        let alignment = (*header).alignment;
        let magic = (*header).magic;

        // prevent double free
        unsafe {
            (*header).size = 0;
            (*header).alignment = 0;
            (*header).magic = 0;
        }

        if magic != MAGIC_NUMBER {
            serial_print!("Invalid magic number: {:?}\n", magic);
            return;
        }

        if let Ok(layout) = Layout::from_size_align(total_size, alignment) {
            let raw_ptr = ((ptr as usize - core::mem::size_of::<AllocationHeader>()) & !(alignment - 1)) as *mut u8;
            serial_print!("Deallocated {:?} bytes at {:p}\n", total_size, raw_ptr);
            Global.deallocate(NonNull::new_unchecked(raw_ptr), layout);
        }
    }
}
