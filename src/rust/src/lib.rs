#![no_std]
#![no_main]
#![feature(str_from_raw_parts)]

#[allow(warnings)]
mod bindings;

mod allocator;
mod interrupts;
mod panic;
mod serial;
