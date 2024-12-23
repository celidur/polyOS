use crate::{
    allocator::{init_heap, serial_print_memory},
    bindings::{
        boot_loadinfo, kernel_init, kernel_init2, process, process_load_switch,
        task_run_first_ever_task,
    },
    entry_point,
};

entry_point!(kernel_main);

fn kernel_main() -> ! {
    unsafe { kernel_init() };

    init_heap();
    unsafe { kernel_init2() };

    unsafe { boot_loadinfo() };

    serial_print_memory();

    let p: *mut *mut process = core::ptr::null_mut();
    let res = unsafe { process_load_switch(c"0:/bin/shell.elf".as_ptr(), p) };
    if res < 0 {
        panic!("Failed to load process");
    }

    unsafe { task_run_first_ever_task() };
}
