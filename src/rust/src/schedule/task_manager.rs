use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use spin::RwLock;

use crate::{bindings::user_registers, error::KernelError};

use super::{
    process::Process,
    task::{Task, TaskId},
};

/// Maximum scheduler priority (0 = highest)
const MAX_PRIORITY: usize = 1;

pub struct TaskManager {
    tasks: BTreeMap<TaskId, RwLock<Task>>,
    ready: [VecDeque<TaskId>; MAX_PRIORITY],
    current: Option<TaskId>,
    next_task_id: TaskId,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            tasks: BTreeMap::new(),
            ready: [VecDeque::new()],
            current: None,
            next_task_id: 0,
        }
    }

    pub fn spawn(&mut self, process: Arc<Process>) -> Result<TaskId, KernelError> {
        let priority = 0;
        let id = self.next_task_id;
        self.next_task_id += 1;
        let task = Task::new(id, process, priority);
        let nn = RwLock::new(task);
        self.tasks.insert(id, nn);
        self.ready[priority].push_back(id);
        if self.current.is_none() {
            self.current = Some(id);
        }
        Ok(id)
    }

    pub fn task_page(&self) -> Result<(), KernelError> {
        if let Some(cur) = self.current
            && let Some(nn_cur) = self.tasks.get(&cur)
        {
            let process = &nn_cur.read().process;
            unsafe { user_registers() };
            process.page_directory.switch();
            return Ok(());
        }
        Err(KernelError::NoTasks)
    }

    pub fn schedule(&mut self) -> Result<(), KernelError> {
        if let Some(cur) = self.current
            && let Some(nn_cur) = self.tasks.get(&cur)
        {
            let prio = nn_cur.read().priority;
            self.ready[prio].push_back(cur);
        }

        for prio in 0..MAX_PRIORITY {
            if let Some(next_id) = self.ready[prio].pop_front() {
                self.current = Some(next_id);
                if let Some(nn_next) = self.tasks.get(&next_id) {
                    let process = &nn_next.read().process;
                    unsafe { user_registers() };
                    process.page_directory.switch();
                    return Ok(());
                }
            }
        }
        Err(KernelError::NoTasks)
    }

    pub fn get_current(&self) -> Option<&RwLock<Task>> {
        if let Some(cur) = self.current {
            self.tasks.get(&cur)
        } else {
            None
        }
    }

    pub fn remove(&mut self, task_id: TaskId) {
        if let Some(task) = self.tasks.remove(&task_id) {
            let prio = task.read().priority;
            self.ready[prio].retain(|&x| x != task_id);
        }
        if self.current == Some(task_id) {
            self.current = None;
        }
    }
}
