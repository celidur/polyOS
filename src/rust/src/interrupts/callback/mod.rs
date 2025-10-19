mod clock;
mod exceptions;

pub use clock::idt_clock;
pub use exceptions::{
    idt_general_protection_fault, idt_handle_exception, idt_handle_exception_error, idt_page_fault,
};
