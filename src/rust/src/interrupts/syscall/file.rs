use core::ffi::c_void;

use alloc::vec::Vec;

use crate::{
    constant::MAX_PATH,
    fs::file::{FileStat, fclose, fopen, fread, fseek, fstat, fwrite},
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::task::{copy_string_from_task, copy_string_to_task},
};

pub fn int80h_command13_fopen(_frame: &InterruptFrame) -> u32 {
    let mut filename: [u8; MAX_PATH] = [0; MAX_PATH];
    let res = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;

        let file_user_ptr = current_task.read().get_stack_item(0);
        if file_user_ptr == 0 {
            return None;
        }

        if copy_string_from_task(
            &current_task.read().process.page_directory,
            file_user_ptr,
            filename.as_mut_ptr() as u32,
            filename.len() as u32,
        )
        .is_err()
        {
            return None;
        }

        Some(filename.as_ptr())
    });

    if res.is_none() {
        let res = u32::MAX;
        return res;
    }

    let mode = "r";
    let filename = unsafe { core::ffi::CStr::from_ptr(res.unwrap() as *const i8) };
    let filename = filename.to_str().unwrap_or("");

    fopen(filename, mode) as u32
}

// TODO: Update this function to be more clean
pub fn int80h_command14_fread(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let fd = current_task.read().get_stack_item(0);
        let file_user_ptr = current_task.read().get_stack_item(1);
        if file_user_ptr == 0 {
            let res = u32::MAX;
            return res;
        }
        let size = current_task.read().get_stack_item(2);

        let mut data: Vec<u8> = vec![0; size as usize];
        let res = fread(fd as i32, data.as_mut_slice());

        let _ = copy_string_to_task(
            &current_task.read().process.page_directory,
            data.as_ptr() as u32,
            file_user_ptr,
            size,
        );
        res as u32
    })
}

pub fn int80h_command15_fwrite(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let fd = current_task.read().get_stack_item(0);
        let ptr = current_task.read().get_stack_item(1);
        if ptr == 0 {
            let res = u32::MAX;
            return res;
        }
        let size = current_task.read().get_stack_item(2);

        let mut data: Vec<u8> = Vec::with_capacity(size as usize);

        let _ = copy_string_from_task(
            &current_task.read().process.page_directory,
            ptr,
            data.as_ptr() as u32,
            size + 1,
        );

        fwrite(fd as i32, data.as_mut_ptr() as *mut c_void, size) as u32
    })
}

pub fn int80h_command16_fseek(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let fd = current_task.read().get_stack_item(0);
        let offset = current_task.read().get_stack_item(1);
        let mode = current_task.read().get_stack_item(2);

        fseek(fd as i32, offset, mode) as u32
    })
}

pub fn int80h_command17_fstat(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let fd = current_task.read().get_stack_item(0);
        let ptr = current_task.read().get_stack_item(1);

        let stat: FileStat = unsafe { core::mem::zeroed() };
        let stat = &stat as *const FileStat as *mut FileStat;
        let res = fstat(fd as i32, stat);

        let _ = copy_string_to_task(
            &current_task.read().process.page_directory,
            stat as u32,
            ptr,
            core::mem::size_of::<FileStat>() as u32,
        );

        res as u32
    })
}

pub fn int80h_command18_fclose(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let fd = current_task.read().get_stack_item(0);

        fclose(fd as i32) as u32
    })
}
