use core::ffi::c_void;

use crate::{allocator::print_memory, bindings, kernel::KERNEL};

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command4_malloc(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return core::ptr::null_mut();
        };
        let size = current_task.read().get_stack_item(0);
        let process = current_task.read().process.clone();
        process.malloc(size as usize)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command5_free(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return core::ptr::null_mut();
        };
        let ptr = current_task.read().get_stack_item(0) as *mut c_void;
        let process = current_task.read().process.clone();
        process.free(ptr);

        core::ptr::null_mut()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command10_print_memory(
    _frame: *mut bindings::interrupt_frame,
) -> *mut c_void {
    print_memory();
    core::ptr::null_mut()
}
