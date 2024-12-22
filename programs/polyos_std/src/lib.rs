#![no_std]

#[allow(unused_imports)] // macros from `alloc` are not used on all platforms
#[macro_use]
extern crate alloc;

#[macro_use]
pub mod stdio;
#[allow(warnings)]
mod bindings;
pub mod cli;
mod memory;
pub mod prelude;
pub mod process;

pub use prelude::*;

pub use alloc::{boxed, collections, format, rc, string, vec};

#[cfg(feature = "macros")]
pub use polyos_std_macros::main;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}\n", info);
    println!("{}", info);
    process::exit(1);
}
