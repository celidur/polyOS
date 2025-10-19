use core::ffi::c_void;

use crate::{allocator::print_memory, interrupts::InterruptFrame, kernel::KERNEL};

pub fn int80h_command4_malloc(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return 0;
        };
        let size = current_task.read().get_stack_item(0);
        let process = current_task.read().process.clone();
        process.malloc(size as usize) as u32
    })
}

pub fn int80h_command5_free(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return 0;
        };
        let ptr = current_task.read().get_stack_item(0) as *mut c_void;
        let process = current_task.read().process.clone();
        process.free(ptr);

        0
    })
}

pub fn int80h_command10_print_memory(_frame: &InterruptFrame) -> u32 {
    print_memory();
    0
}
