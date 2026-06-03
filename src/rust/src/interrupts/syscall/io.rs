use crate::{
    constant::TIMER_HZ,
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::task::{task_current_set_return_value, task_next},
};

use super::abi;

pub fn syscall_sleep(_frame: &InterruptFrame) -> u32 {
    let duration_ms = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(0) as u64)
    });

    let Some(duration_ms) = duration_ms else {
        return abi::error();
    };

    if duration_ms == 0 {
        return 0;
    }

    let sleep_ticks = duration_ms
        .saturating_mul(TIMER_HZ as u64)
        .saturating_add(999)
        / 1000;

    let sleep_set = KERNEL.with_task_manager(|tm| {
        let now = tm.get_tick();
        tm.sleep_current_until(now.saturating_add(sleep_ticks.max(1)))
            .is_ok()
    });

    if !sleep_set {
        return abi::error();
    }

    task_current_set_return_value(0);
    task_next();
}

pub fn syscall_ioctl(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, request, arg)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
            task.get_stack_item(2),
        ))
    }) else {
        return abi::error();
    };

    match process.get_fd(fd) {
        Some(descriptor) => match descriptor.ioctl(request, arg, &process.page_directory) {
            Ok(result) => result,
            Err(_) => abi::error(),
        },
        _ => abi::error(),
    }
}
