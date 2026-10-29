use super::abi;
use crate::{
    kernel::KERNEL,
    schedule::{
        legacy_semaphore::SEMAPHORES,
        task::{task_current_set_return_value, task_next},
    },
};
pub fn syscall_semaphore_create(initial_count: u32) -> u32 {
    let Some(count) = KERNEL.with_task_manager(|tm| {
        tm.get_current()?;
        Some(initial_count as i32)
    }) else {
        return abi::errno(abi::ESRCH);
    };

    if count < 0 {
        return abi::errno(abi::EINVAL);
    }

    SEMAPHORES.lock().create(count as isize) as u32
}

pub fn syscall_semaphore_wait(semid: u32) -> u32 {
    let Some(id) = KERNEL.with_task_manager(|tm| {
        tm.get_current()?;
        Some(semid as usize)
    }) else {
        return abi::errno(abi::ESRCH);
    };

    let wait_result = KERNEL.with_task_manager(|tm| SEMAPHORES.lock().wait(id, tm));

    match wait_result {
        Ok(true) => 0,
        Ok(false) => {
            task_current_set_return_value(0);
            task_next();
        }
        Err(errno) => abi::errno(errno),
    }
}

pub fn syscall_semaphore_signal(semid: u32) -> u32 {
    let Some(id) = KERNEL.with_task_manager(|tm| {
        tm.get_current()?;
        Some(semid as usize)
    }) else {
        return abi::errno(abi::ESRCH);
    };

    match KERNEL.with_task_manager(|tm| SEMAPHORES.lock().signal(id, tm)) {
        Ok(()) => 0,
        Err(errno) => abi::errno(errno),
    }
}

pub fn syscall_semaphore_close(semid: u32) -> u32 {
    let Some(id) = KERNEL.with_task_manager(|tm| {
        tm.get_current()?;
        Some(semid as usize)
    }) else {
        return abi::errno(abi::ESRCH);
    };

    match KERNEL.with_task_manager(|tm| SEMAPHORES.lock().close(id, tm)) {
        Ok(()) => 0,
        Err(errno) => abi::errno(errno),
    }
}
