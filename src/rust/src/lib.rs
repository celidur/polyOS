#![no_std]
#![no_main]

#[allow(warnings)]
mod bindings;

mod allocator;
mod interrupts;
mod panic;
mod serial;
