use crate::interrupts;
use core::ffi::{c_char, c_void};
use core::str;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::kernel::KERNEL;

use super::vfs::FileHandle;

#[repr(C)]
pub struct FileStat {
    pub size: u32,
    pub flags: u32,
}

pub const FILE_SEEK_SET: u32 = 0;
// pub const FILE_SEEK_CUR: u32 = 1;
// pub const FILE_SEEK_END: u32 = 2;

// pub const FILE_MODE_READ: u32 = 0;
// pub const FILE_MODE_WRITE: u32 = 1;
// pub const FILE_MODE_APPEND: u32 = 2;
// pub const FILE_MODE_INVALID: u32 = 3;

// pub const FILE_STAT_READ_ONLY: u32 = 0b00000001;
const MAX_FD: usize = 128;

lazy_static! {
    static ref FILE_TABLE: Mutex<[Option<FileHandle>; MAX_FD]> =
        Mutex::new([const { None }; MAX_FD]);
}

// Converts C `char*` to Rust `&str`
fn c_str_to_str(ptr: *const c_char) -> Option<&'static str> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(ptr as *const u8, len)).ok()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fopen(filename: *const c_char, _mode: *const c_char) -> i32 {
    let Some(path) = c_str_to_str(filename) else {
        return -1;
    };

    interrupts::without_interrupts(|| {
        let mut table = FILE_TABLE.lock();

        for (fd, slot) in table.iter_mut().enumerate() {
            if slot.is_none() {
                let handle = KERNEL.vfs.read().open(path);
                if let Ok(file) = handle {
                    *slot = Some(file);
                    return (fd + 1) as i32;
                } else {
                    return -2;
                }
            }
        }

        -3 // no slots
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fread(fd: i32, ptr: *mut c_void, size: u32) -> i32 {
    let fd = fd - 1;
    if fd < 0 || fd as usize >= MAX_FD {
        return -1;
    }

    let buf = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, size as usize) };

    interrupts::without_interrupts(|| {
        let mut table = FILE_TABLE.lock();
        let Some(Some(file)) = table.get_mut(fd as usize) else {
            return -2;
        };

        match file.ops.read(buf) {
            Ok(read) => read as i32,
            Err(_) => -3,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fseek(fd: i32, offset: u32, mode: u32) -> i32 {
    let fd = fd - 1;
    if fd < 0 || fd as usize >= MAX_FD {
        return -1;
    }

    interrupts::without_interrupts(|| {
        let mut table = FILE_TABLE.lock();
        let Some(Some(file)) = table.get_mut(fd as usize) else {
            return -2;
        };

        let result = match mode {
            FILE_SEEK_SET => file.ops.seek(offset as usize),
            _ => return -1,
        };

        match result {
            Ok(_) => 0,
            Err(_) => -3,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fstat(fd: i32, stat: *mut FileStat) -> i32 {
    let fd = fd - 1;
    if fd < 0 || fd as usize >= MAX_FD || stat.is_null() {
        return -1;
    }

    interrupts::without_interrupts(|| {
        let table = FILE_TABLE.lock();
        let Some(Some(file)) = table.get(fd as usize) else {
            return -2;
        };

        let result = file.ops.stat();

        match result {
            Ok(meta) => {
                unsafe {
                    (*stat).size = meta.size as u32;
                    (*stat).flags = 0;
                }
                0
            }
            Err(_) => -3,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fwrite(fd: i32, ptr: *mut c_void, size: u32) -> i32 {
    let fd = fd - 1;
    if fd < 0 || fd as usize >= MAX_FD {
        return -1;
    }

    let buf = unsafe { core::slice::from_raw_parts(ptr as *const u8, size as usize) };

    interrupts::without_interrupts(|| {
        let mut table = FILE_TABLE.lock();
        let Some(Some(file)) = table.get_mut(fd as usize) else {
            return -2;
        };

        match file.ops.write(buf) {
            Ok(written) => written as i32,
            Err(_) => -3,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fclose(fd: i32) -> i32 {
    let fd = fd - 1;
    if fd < 0 || fd as usize >= 128 {
        return -1;
    }

    interrupts::without_interrupts(|| {
        let mut table = FILE_TABLE.lock();
        let Some(slot) = table.get_mut(fd as usize) else {
            return -2;
        };

        *slot = None;
        0
    })
}
