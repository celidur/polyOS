use core::arch::{asm, naked_asm};

use crate::{
    interrupts::{
        interrupt::InterruptSource,
        interrupt_frame::{InterruptErrorFrame, InterruptFrame},
        register::RegisterInterrupt,
        syscall::syscall_handle,
        utils::eoi_irq,
    },
    kernel::KERNEL,
    schedule::task::{task_current_save_state, task_page},
};

#[unsafe(no_mangle)]
pub extern "C" fn interrupt_handler(interrupt: u32, frame: &InterruptFrame) {
    let previous_cr3 = current_page_directory();
    KERNEL.kernel_page();
    if frame.is_user_mode() {
        task_current_save_state(frame);
    }
    eoi_irq(interrupt);

    if let InterruptSource::Plain(int) = InterruptSource::new(interrupt as u16) {
        if let Some(cb) = int.get_device_callback() {
            cb.interrupt();
        } else if let Some(cb) = int.get_callback() {
            cb(frame);
        }
    }

    restore_page_directory(previous_cr3);
}

#[unsafe(no_mangle)]
pub extern "C" fn interrupt_handler_error(
    error_code: u32,
    interrupt: u32,
    frame: &InterruptErrorFrame,
) {
    let previous_cr3 = current_page_directory();
    let frame = frame.without_error_code();
    KERNEL.kernel_page();
    if frame.is_user_mode() {
        task_current_save_state(&frame);
    }
    if let InterruptSource::Error(int) = InterruptSource::new(interrupt as u16)
        && let Some(cb) = int.get_callback()
    {
        cb(&frame, error_code);
    }

    restore_page_directory(previous_cr3);
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(frame: &mut InterruptFrame) -> u32 {
    KERNEL.kernel_page();
    task_current_save_state(frame);
    KERNEL.with_task_manager(|tm| {
        if let Some(t) = tm.get_current() {
            t.write().restarting = false;
        }
    });
    let res = syscall_handle(frame);
    frame.eax = res;
    task_current_save_state(frame);
    if crate::schedule::signal::prepare_current() || KERNEL.with_task_manager(|tm|tm.get_current().is_none_or(|t|t.read().stopped)) {
        crate::schedule::task::task_next();
    }
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
        cld
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

fn current_page_directory() -> u32 {
    let cr3: u32;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}
fn restore_page_directory(cr3: u32) {
    unsafe {
        asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
    }
}
