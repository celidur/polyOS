use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    kernel::KERNEL,
    schedule::{
        semaphore::Semaphore,
        task::{task_current_set_return_value, task_next},
        task_manager::TaskManager,
    },
};

use super::abi;

lazy_static! {
    static ref SEMAPHORES: Mutex<SemaphoreTable> = Mutex::new(SemaphoreTable::new());
}

struct SemaphoreTable {
    next_id: usize,
    semaphores: BTreeMap<usize, Semaphore>,
}

impl SemaphoreTable {
    fn new() -> Self {
        Self {
            next_id: 1,
            semaphores: BTreeMap::new(),
        }
    }

    fn create(&mut self, count: isize) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1).max(1);
        self.semaphores.insert(id, Semaphore::new(id, count));
        id
    }

    fn wait(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<bool, i32> {
        self.semaphores
            .get_mut(&id)
            .ok_or(abi::EINVAL)?
            .wait(task_manager)
            .map_err(|_| abi::ESRCH)
    }

    fn signal(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<(), i32> {
        self.semaphores
            .get_mut(&id)
            .ok_or(abi::EINVAL)
            .map(|semaphore| semaphore.signal(task_manager))
    }

    fn close(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<(), i32> {
        self.semaphores
            .remove(&id)
            .ok_or(abi::EINVAL)
            .map(|mut semaphore| semaphore.close(task_manager, abi::errno(abi::EINVAL)))
    }
}

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
