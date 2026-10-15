use crate::{
    interrupts::interrupt_frame::InterruptFrame, kernel::KERNEL, schedule::task::task_next,
};

pub fn idt_clock(frame: &InterruptFrame) {
    KERNEL.kernel_page();
    KERNEL.with_task_manager(|tm| tm.tick());

    if frame.is_user_mode() {
        task_next();
    }
}
