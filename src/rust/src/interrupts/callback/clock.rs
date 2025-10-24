use crate::{
    interrupts::interrupt_frame::InterruptFrame, kernel::KERNEL,
    schedule::task::task_current_save_state, utils::sync,
};

pub fn idt_clock(frame: &InterruptFrame) {
    KERNEL.kernel_page();
    task_current_save_state(frame);

    sync();
}
