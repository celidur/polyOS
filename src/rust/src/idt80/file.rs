use core::ffi::c_void;

use alloc::vec::Vec;

use crate::{
    bindings::{self, MAX_PATH, copy_string_from_task, copy_string_to_task},
    fs::file::{FileStat, fclose, fopen, fread, fseek, fstat, fwrite},
    kernel::KERNEL,
};

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command13_fopen(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    let res = KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return None;
        };

        let file_user_ptr = current_task.read().get_stack_item(0);
        if file_user_ptr == 0 {
            return None;
        }

        let mut filename: [u8; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let res = unsafe {
            copy_string_from_task(
                current_task.read().process.page_directory as *mut u32,
                file_user_ptr as *mut c_void,
                filename.as_mut_ptr() as *mut c_void,
                filename.len() as i32,
            )
        };
        if res != 0 {
            return None;
        }

        Some(filename.as_ptr())
    });

    if res.is_none() {
        let res = -1;
        return res as *mut c_void;
    }
    let filename = res.unwrap();

    let mode = "r";

    fopen(filename as *const i8, mode.as_ptr() as *const i8) as *mut c_void
}

// TODO: Update this function to be more clean
#[unsafe(no_mangle)]
pub extern "C" fn int80h_command14_fread(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = -1;
            return res as *mut c_void;
        };

        let fd = current_task.read().get_stack_item(0);
        let file_user_ptr = current_task.read().get_stack_item(1);
        if file_user_ptr == 0 {
            let res = -1;
            return res as *mut c_void;
        }
        let size = current_task.read().get_stack_item(2);

        let mut data: Vec<u8> = Vec::with_capacity(size as usize);
        let res = fread(fd as i32, data.as_mut_ptr() as *mut c_void, size);

        let _ = unsafe {
            copy_string_to_task(
                current_task.read().process.page_directory as *mut u32,
                data.as_ptr() as *mut c_void,
                file_user_ptr as *mut c_void,
                size,
            ) as *mut c_void
        };
        res as *mut c_void
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command15_fwrite(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = -1;
            return res as *mut c_void;
        };

        let fd = current_task.read().get_stack_item(0);
        let ptr = current_task.read().get_stack_item(1);
        if ptr == 0 {
            let res = -1;
            return res as *mut c_void;
        }
        let size = current_task.read().get_stack_item(2);

        let mut data: Vec<u8> = Vec::with_capacity(size as usize);

        unsafe {
            copy_string_from_task(
                current_task.read().process.page_directory as *mut u32,
                ptr as *mut c_void,
                data.as_ptr() as *mut c_void,
                size as i32 + 1,
            )
        };

        fwrite(fd as i32, data.as_mut_ptr() as *mut c_void, size) as *mut c_void
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command16_fseek(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = -1;
            return res as *mut c_void;
        };

        let fd = current_task.read().get_stack_item(0);
        let offset = current_task.read().get_stack_item(1);
        let mode = current_task.read().get_stack_item(2);

        fseek(fd as i32, offset, mode) as *mut c_void
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command17_fstat(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = -1;
            return res as *mut c_void;
        };

        let fd = current_task.read().get_stack_item(0);
        let ptr = current_task.read().get_stack_item(1);

        let stat: FileStat = unsafe { core::mem::zeroed() };
        let stat = &stat as *const FileStat as *mut FileStat;
        let res = fstat(fd as i32, stat);

        let _ = unsafe {
            copy_string_to_task(
                current_task.read().process.page_directory as *mut u32,
                stat as *mut c_void,
                ptr as *mut c_void,
                core::mem::size_of::<bindings::file_stat>() as u32,
            ) as *mut c_void
        };

        res as *mut c_void
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command18_fclose(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = -1;
            return res as *mut c_void;
        };

        let fd = current_task.read().get_stack_item(0);

        fclose(fd as i32) as *mut c_void
    })
}
