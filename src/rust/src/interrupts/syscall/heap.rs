use crate::{kernel::KERNEL, memory::print_memory};

pub fn syscall_brk(requested_break: u32) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let Some(current_task) = tm.get_current() else {
            return 0;
        };

        current_task
            .read()
            .process
            .set_program_break(requested_break)
    })
}

pub fn syscall_print_memory() -> u32 {
    print_memory();
    0
}
