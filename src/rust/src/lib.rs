#![no_std]
#![no_main]
#![feature(str_from_raw_parts)]
#![feature(allocator_api)]

extern crate alloc;

#[allow(warnings)]
mod bindings;

mod allocator;
mod interrupts;
mod panic;
mod serial;
mod memory;
mod kernel_main;

mod macros;