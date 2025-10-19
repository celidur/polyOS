use core::ffi::c_void;

use alloc::vec::Vec;

use crate::{
    bindings::copy_string_from_task,
    interrupts::idt::InterruptFrame,
    kernel::KERNEL,
    print::{clear_screen, terminal_writechar},
    serial_print,
};

pub fn int80h_command0_serial(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let ptr = current_task.read().get_stack_item(0);
        if ptr == 0 {
            let res = u32::MAX;
            return res;
        }
        let size = 1025;

        let mut data: Vec<u8> = Vec::with_capacity(size as usize);

        unsafe {
            copy_string_from_task(
                current_task.read().process.page_directory as *mut u32,
                ptr as *mut c_void,
                data.as_ptr() as *mut c_void,
                size,
            )
        };

        unsafe { data.set_len(size as usize) };
        data.push(0);

        let data = unsafe { core::slice::from_raw_parts(data.as_ptr(), size as usize) };
        let data = unsafe { core::ffi::CStr::from_ptr(data.as_ptr() as *const i8) };
        let data = data.to_str().unwrap_or("");

        serial_print!("{}", data);

        0
    })
}

pub fn int80h_command1_print(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let ptr = current_task.read().get_stack_item(0);
        if ptr == 0 {
            let res = u32::MAX;
            return res;
        }
        let size = 1025;

        let mut data: Vec<u8> = Vec::with_capacity(size as usize);

        unsafe {
            copy_string_from_task(
                current_task.read().process.page_directory as *mut u32,
                ptr as *mut c_void,
                data.as_ptr() as *mut c_void,
                size,
            )
        };

        unsafe { data.set_len(size as usize) };
        data.push(0);

        let data = unsafe { core::slice::from_raw_parts(data.as_ptr(), size as usize) };
        let data = unsafe { core::ffi::CStr::from_ptr(data.as_ptr() as *const i8) };
        let data = data.to_str().unwrap_or("");

        print!("{}", data);

        0
    })
}

pub fn int80h_command2_getkey(_frame: &InterruptFrame) -> u32 {
    let c = KERNEL.keyboard_pop();
    if let Some(c) = c {
        return c as u32;
    }

    0
}

pub fn int80h_command3_putchar(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = u32::MAX;
            return res;
        };

        let c = current_task.read().get_stack_item(0) as u8;
        terminal_writechar(c, 15);

        0
    })
}

pub fn int80h_command11_remove_last_char(_frame: &InterruptFrame) -> u32 {
    terminal_writechar(0x08, 15);
    0
}

pub fn int80h_command12_clear_screen(_frame: &InterruptFrame) -> u32 {
    clear_screen();
    0
}
