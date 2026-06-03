mod callback;
mod handler;
mod idt;
mod interrupt;
mod interrupt_frame;
mod irq_numbers;
mod register;
mod syscall;
mod utils;

#[allow(unused_imports)]
pub use interrupt::{InterruptHandlerKind, InterruptSource};
pub use interrupt_frame::InterruptFrame;
pub use register::InterruptDevice;
pub use utils::{disable_interrupts, enable_interrupts, without_interrupts};

pub fn interrupts_init() {
    idt::idt_init();
    syscall::syscall_init();
}
