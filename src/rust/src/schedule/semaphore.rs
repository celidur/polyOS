#![allow(dead_code)]

use alloc::collections::VecDeque;

use crate::error::KernelError;

use super::{
    task::{TaskId, WaitReason},
    task_manager::TaskManager,
};

pub struct Semaphore {
    id: usize,
    count: isize,
    waiters: VecDeque<TaskId>,
}

impl Semaphore {
    pub fn new(id: usize, count: isize) -> Self {
        Self {
            id,
            count,
            waiters: VecDeque::new(),
        }
    }

    pub fn count(&self) -> isize {
        self.count
    }

    pub fn wait(&mut self, task_manager: &mut TaskManager) -> Result<bool, KernelError> {
        if self.count > 0 {
            self.count -= 1;
            return Ok(true);
        }

        let task_id = task_manager.block_current(WaitReason::Semaphore(self.id))?;
        if !self.waiters.contains(&task_id) {
            self.waiters.push_back(task_id);
        }
        Ok(false)
    }

    pub fn signal(&mut self, task_manager: &mut TaskManager) {
        while let Some(task_id) = self.waiters.pop_front() {
            if task_manager.wake_task_with_return_value(task_id, 0).is_ok() {
                return;
            }
        }

        self.count = self.count.saturating_add(1);
    }

    pub fn close(&mut self, task_manager: &mut TaskManager, return_value: u32) {
        while let Some(task_id) = self.waiters.pop_front() {
            let _ = task_manager.wake_task_with_return_value(task_id, return_value);
        }
    }
}
