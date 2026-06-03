use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    interrupts::InterruptFrame,
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

    fn wait(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<bool, ()> {
        self.semaphores
            .get_mut(&id)
            .ok_or(())?
            .wait(task_manager)
            .map_err(|_| ())
    }

    fn signal(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<(), ()> {
        self.semaphores
            .get_mut(&id)
            .ok_or(())
            .map(|semaphore| semaphore.signal(task_manager))
    }

    fn close(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<(), ()> {
        self.semaphores
            .remove(&id)
            .ok_or(())
            .map(|mut semaphore| semaphore.close(task_manager, abi::error()))
    }
}

pub fn syscall_semaphore_create(_frame: &InterruptFrame) -> u32 {
    let Some(count) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(0) as i32)
    }) else {
        return abi::error();
    };

    if count < 0 {
        return abi::error();
    }

    SEMAPHORES.lock().create(count as isize) as u32
}

pub fn syscall_semaphore_wait(_frame: &InterruptFrame) -> u32 {
    let Some(id) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(0) as usize)
    }) else {
        return abi::error();
    };

    let wait_result = KERNEL.with_task_manager(|tm| SEMAPHORES.lock().wait(id, tm));

    match wait_result {
        Ok(true) => 0,
        Ok(false) => {
            task_current_set_return_value(0);
            task_next();
        }
        Err(_) => abi::error(),
    }
}

pub fn syscall_semaphore_signal(_frame: &InterruptFrame) -> u32 {
    let Some(id) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(0) as usize)
    }) else {
        return abi::error();
    };

    match KERNEL.with_task_manager(|tm| SEMAPHORES.lock().signal(id, tm)) {
        Ok(()) => 0,
        Err(()) => abi::error(),
    }
}

pub fn syscall_semaphore_close(_frame: &InterruptFrame) -> u32 {
    let Some(id) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(0) as usize)
    }) else {
        return abi::error();
    };

    match KERNEL.with_task_manager(|tm| SEMAPHORES.lock().close(id, tm)) {
        Ok(()) => 0,
        Err(()) => abi::error(),
    }
}
