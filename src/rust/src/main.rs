#![no_std]
#![no_main]
#![feature(str_from_raw_parts)]
#![feature(allocator_api)]
extern crate alloc;

#[macro_use]
mod macros;

mod c_shims;
mod constant;
mod device;
mod entrypoint;
mod error;
mod fs;
mod gdt;
mod interrupts;
mod kernel;
mod loader;
mod memory;
mod panic;
mod print;
mod schedule;
mod serial;
mod tss;
mod utils;

use crate::{
    device::{
        keyboard::KEYBOARD,
        pci::pci_read_config,
        screen::{ScreenMode, TextMode},
    },
    gdt::GDT,
    interrupts::interrupts_init,
    kernel::KERNEL,
    memory::{enable_paging, init_heap, serial_print_memory},
    schedule::task::task_next,
    utils::boot_image,
};

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
    lazy_static::initialize(&GDT);
    init_heap();

    KERNEL.set_mode(ScreenMode::Text(TextMode::Text90x60));
    KERNEL.init_rootfs();
    KERNEL.init_page();

    interrupts_init();

    KEYBOARD.lock().init();

    GDT.write().init_gdt();

    KERNEL.kernel_page();
    enable_paging();

    boot_image();

    serial_print_memory();

    list_pci_devices();

    serial_println!("Kernel main: spawning shell-v2.elf");

    let _ = KERNEL.with_process_manager(|pm| pm.spawn("/bin/shell-v2.elf", None, None));

    task_next();

    panic!("No more tasks to run\n");
}
