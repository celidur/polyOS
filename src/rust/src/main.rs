#![no_std]
#![no_main]
extern crate alloc;

#[macro_use]
mod macros;

mod constant;
mod device;
mod entrypoint;
mod error;
mod fs;
mod gdt;
mod interrupts;
mod kernel;
mod kernel_selftest;
mod memory;
mod net;
mod panic;
mod print;
mod schedule;
mod serial;
mod tss;
mod utils;

use crate::{
    device::{
        pci::pci_read_config,
        screen::{SCREEN_DRIVER, ScreenMode, TextMode},
    },
    gdt::GDT,
    interrupts::interrupts_init,
    kernel::KERNEL,
    memory::{enable_paging, serial_print_memory},
    schedule::task::task_next,
    utils::boot_image,
};

use alloc::format;

fn list_pci_devices() {
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                let vendor_id = unsafe { pci_read_config(bus, device, function, 0x00) } & 0xFFFF;
                let device_id =
                    (unsafe { pci_read_config(bus, device, function, 0x00) } >> 16) & 0xFFFF;
                if vendor_id != 0xFFFF {
                    serial_println!(
                        "PCI Device: Bus {} Device {} Function {} - Vendor: {:#X}, Device: {:#X}",
                        bus,
                        device,
                        function,
                        vendor_id,
                        device_id
                    );
                }
            }
        }
    }
}

pub fn kernel_main() -> ! {
    lazy_static::initialize(&KERNEL);

    SCREEN_DRIVER.set_mode(ScreenMode::Text(TextMode::Text90x60));

    interrupts_init();

    GDT.init_gdt();

    KERNEL.kernel_page();
    enable_paging();

    boot_image();

    serial_print_memory();

    list_pci_devices();

    let start_path = "/bin/shell-v2.elf";

    serial_println!("Kernel main: spawning {}", start_path);

    KERNEL
        .with_process_manager(|pm| pm.spawn(start_path, None, None))
        .expect(&format!("failed to spawn {}", start_path));

    task_next();
}
