extern crate alloc;

use core::panic::PanicInfo;

use crate::{
    device::screen::Color,
    interrupts::disable_interrupts,
    print::{disable_cursor, set_color},
    utils::halt,
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    disable_interrupts();
    set_color(Color::Black, Color::LightRed);
    print!("\nKERNEL PANIC: ");
    set_color(Color::Black, Color::Red);
    println!("{}", info);
    disable_cursor();
    serial_println!("KERNEL PANIC: {}", info);
    halt();
}
