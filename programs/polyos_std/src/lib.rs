#![no_std]

extern crate alloc;

#[macro_use]
pub mod stdio;
#[allow(warnings)]
mod bindings;
mod memory;
pub mod entry;

pub use alloc::{boxed, format, string, vec, rc};

#[cfg(feature = "macros")]
pub use polyos_std_macros::main;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}\n", info);
    println!("{}", info);
    entry::exit(1);
}
