use core::arch::naked_asm;

use crate::{
    interrupts::{
        interrupt::InterruptSource, interrupt_frame::InterruptFrame, register::RegisterInterrupt,
        syscall::syscall_handle, utils::eoi_pic1,
    },
    kernel::KERNEL,
    schedule::task::{task_current_save_state, task_page},
};

#[unsafe(no_mangle)]
pub extern "C" fn interrupt_handler(interrupt: u32, frame: &InterruptFrame) {
    KERNEL.kernel_page();
    task_current_save_state(frame);
    if let InterruptSource::Plain(int) = InterruptSource::new(interrupt as u16)
        && let Some(cb) = int.get_callback()
    {
        cb(frame);
    }

    task_page();
    eoi_pic1();
}

#[unsafe(no_mangle)]
pub extern "C" fn interrupt_handler_error(error_code: u32, interrupt: u32, frame: &InterruptFrame) {
    KERNEL.kernel_page();
    task_current_save_state(frame);
    if let InterruptSource::Error(int) = InterruptSource::new(interrupt as u16)
        && let Some(cb) = int.get_callback()
    {
        cb(frame, error_code);
    }

    task_page();
    eoi_pic1();
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(frame: &mut InterruptFrame) -> u32 {
    KERNEL.kernel_page();
    task_current_save_state(frame);
    let res = syscall_handle(frame);
    frame.eax = res;
    task_current_save_state(frame);
    task_page();
    res
}

#[unsafe(naked)]
pub extern "C" fn syscall_wrapper() {
    #[allow(unused_unsafe)]
    unsafe {
        naked_asm!(
            "
        pushad
        push esp
        call syscall_handler
        add esp, 4
        mov [esp + 28], eax
        popad
        iretd
        ",
        );
    }
}

pub unsafe extern "C" fn default_handler() {
    panic!("Unhandled interrupt");
}
