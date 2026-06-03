use crate::{
    device::network, interrupts::interrupt_frame::InterruptFrame, kernel::KERNEL,
    schedule::task::task_next,
};

pub fn idt_clock(_frame: &InterruptFrame) {
    KERNEL.kernel_page();
    KERNEL.with_task_manager(|tm| tm.tick());
    network::poll();

    task_next();
}
