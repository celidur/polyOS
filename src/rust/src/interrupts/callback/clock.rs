use crate::{
    bindings::kernel_page, interrupts::interrupt_frame::InterruptFrame,
    schedule::task::task_current_save_state, utils::sync,
};

pub fn idt_clock(frame: &InterruptFrame) {
    unsafe { kernel_page() };
    task_current_save_state(frame);

    sync();
}
