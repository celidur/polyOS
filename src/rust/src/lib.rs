#![no_std]
#![no_main]
#![feature(str_from_raw_parts)]
#![feature(allocator_api)]
extern crate alloc;

#[macro_use]
mod macros;

#[allow(warnings)]
mod bindings;

mod allocator;
mod device;
mod error;
mod fs;
mod idt80;
mod interrupts;
mod kernel;
mod kernel_main;
mod loader;
mod memory;
mod panic;
mod print;
mod serial;
mod task;
mod utils;
