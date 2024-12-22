
use crate::{ bindings::{boot_loadinfo, kernel_init, kfree, process, process_load_switch, task_run_first_ever_task}, entry_point, memory::{self}, serial_println};

entry_point!(kernel_main);

fn kernel_main() -> ! {
    // init_heap();
    serial_println!("Heap initialized");
    unsafe { kernel_init() };
    serial_println!("Kernel initialized");

    unsafe { boot_loadinfo() };

    // print_memory_usage();

    let p: *mut *mut process = core::ptr::null_mut();
    let res = unsafe { process_load_switch(c"0:/bin/shell.elf".as_ptr(), p) };
    if res < 0 {
        panic!("Failed to load process");
    }

    unsafe { task_run_first_ever_task() };
}
