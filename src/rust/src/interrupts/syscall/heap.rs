use crate::{interrupts::InterruptFrame, kernel::KERNEL, memory::print_memory};

pub fn syscall_brk(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let Some(current_task) = tm.get_current() else {
            return 0;
        };

        let requested_break = current_task.read().get_stack_item(0);
        current_task
            .read()
            .process
            .set_program_break(requested_break)
    })
}

pub fn syscall_print_memory(_frame: &InterruptFrame) -> u32 {
    print_memory();
    0
}
