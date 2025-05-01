use alloc::{
    format,
    string::{String, ToString},
};

use crate::{
    allocator::{init_heap, serial_print_memory}, bindings::{
        boot_loadinfo, kernel_init, kernel_init2, process, process_load_switch, task_run_first_ever_task
    }, device::{bufstream::BufStream, pci::pci_read_config}, entry_point, interrupts, kernel::KERNEL, serial_println
};

entry_point!(kernel_main);

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

fn kernel_main() -> ! {
    unsafe { kernel_init() };

    init_heap();
    KERNEL.init_rootfs();

    unsafe { kernel_init2() };

    unsafe { boot_loadinfo() };


    serial_print_memory();

    list_pci_devices();

    let p: *mut *mut process = core::ptr::null_mut();
    let res = unsafe { process_load_switch(c"/bin/shell-v2.elf".as_ptr(), p) };
    if res < 0 {
        panic!("Failed to load process");
    }

    unsafe { task_run_first_ever_task() };
}
