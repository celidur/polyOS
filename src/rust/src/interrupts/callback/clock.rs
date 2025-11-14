use crate::{
    interrupts::{interrupt_frame::InterruptFrame, utils::eoi_pic1},
    kernel::KERNEL,
    schedule::task::{task_current_save_state, task_next},
};

pub fn idt_clock(frame: &InterruptFrame) {
    KERNEL.kernel_page();
    task_current_save_state(frame);

    KERNEL.sync();
    eoi_pic1();

    task_next();
}
