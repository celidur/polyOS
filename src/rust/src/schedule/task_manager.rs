use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec::Vec,
};
use spin::RwLock;

use crate::{error::KernelError, schedule::task::user_registers};

use super::{
    process::Process,
    task::{Registers, Task, TaskId, TaskState, WaitReason},
};

/// Maximum scheduler priority (0 = highest)
const MAX_PRIORITY: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleOutcome {
    Switched,
    Idle,
    NoTasks,
}

pub struct TaskManager {
    tasks: BTreeMap<TaskId, RwLock<Task>>,
    ready: [VecDeque<TaskId>; MAX_PRIORITY],
    current: Option<TaskId>,
    next_task_id: TaskId,
    tick: u64,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            tasks: BTreeMap::new(),
            ready: [VecDeque::new()],
            current: None,
            next_task_id: 0,
            tick: 0,
        }
    }

    pub fn spawn(&mut self, process: Arc<Process>) -> Result<TaskId, KernelError> {
        let priority = 0;
        let id = self.next_task_id;
        self.next_task_id += 1;
        let task = Task::new(id, process, priority);
        let nn = RwLock::new(task);
        self.tasks.insert(id, nn);
        self.queue_ready(id, priority);
        Ok(id)
    }

    pub fn spawn_with_registers(
        &mut self,
        process: Arc<Process>,
        registers: Registers,
        priority: usize,
    ) -> Result<TaskId, KernelError> {
        let priority = priority.min(MAX_PRIORITY - 1);
        let id = self.next_task_id;
        self.next_task_id += 1;
        let task = Task::from_registers(id, process, priority, registers);
        self.tasks.insert(id, RwLock::new(task));
        self.queue_ready(id, priority);
        Ok(id)
    }

    pub fn task_page(&self) -> Result<(), KernelError> {
        if let Some(cur) = self.current
            && let Some(nn_cur) = self.tasks.get(&cur)
        {
            let process = &nn_cur.read().process;
            user_registers();
            process.page_directory.switch();
            return Ok(());
        }
        Err(KernelError::NoTasks)
    }

    pub fn schedule(&mut self) -> ScheduleOutcome {
        if let Some(cur) = self.current
            && let Some(nn_cur) = self.tasks.get(&cur)
        {
            let prio = {
                let task = nn_cur.read();
                task.is_runnable().then_some(task.priority)
            };
            if let Some(prio) = prio {
                self.queue_ready(cur, prio);
            }
        }

        for prio in 0..MAX_PRIORITY {
            while let Some(next_id) = self.ready[prio].pop_front() {
                let Some(nn_next) = self.tasks.get(&next_id) else {
                    continue;
                };

                let process = {
                    let task = nn_next.read();
                    if !task.is_runnable() {
                        continue;
                    }

                    task.process.clone()
                };

                self.current = Some(next_id);
                user_registers();
                process.page_directory.switch();
                return ScheduleOutcome::Switched;
            }
        }

        self.current = None;

        if let Some((task_id, process)) = self.find_runnable_task() {
            self.current = Some(task_id);
            user_registers();
            process.page_directory.switch();
            return ScheduleOutcome::Switched;
        }

        if self.has_waiting_tasks() {
            ScheduleOutcome::Idle
        } else {
            ScheduleOutcome::NoTasks
        }
    }

    pub fn get_current(&self) -> Option<&RwLock<Task>> {
        if let Some(cur) = self.current {
            self.tasks.get(&cur)
        } else {
            None
        }
    }

    pub fn get(&self, task_id: TaskId) -> Option<&RwLock<Task>> {
        self.tasks.get(&task_id)
    }

    pub fn remove(&mut self, task_id: TaskId) {
        if let Some(task) = self.tasks.remove(&task_id) {
            let prio = task.read().priority.min(MAX_PRIORITY - 1);
            self.ready[prio].retain(|&x| x != task_id);
        }
        if self.current == Some(task_id) {
            self.current = None;
        }
    }

    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);

        let now = self.tick;
        let mut wake: Vec<(TaskId, usize)> = Vec::new();
        for (task_id, nn_task) in &self.tasks {
            let mut task = nn_task.write();
            if let TaskState::Sleeping { wake_tick } = task.state
                && now >= wake_tick
            {
                task.state = TaskState::Runnable;
                wake.push((*task_id, task.priority));
            }
        }

        for (task_id, prio) in wake {
            self.queue_ready(task_id, prio);
        }
    }

    pub fn sleep_current_until(&mut self, wake_tick: u64) -> Result<(), KernelError> {
        let Some(cur) = self.current else {
            return Err(KernelError::NoTasks);
        };

        let Some(nn_task) = self.tasks.get(&cur) else {
            self.current = None;
            return Err(KernelError::NoTasks);
        };

        let mut task = nn_task.write();
        task.state = TaskState::Sleeping {
            wake_tick: wake_tick.max(self.tick.wrapping_add(1)),
        };
        Ok(())
    }

    #[allow(dead_code)]
    pub fn block_current(&mut self, reason: WaitReason) -> Result<TaskId, KernelError> {
        let Some(cur) = self.current else {
            return Err(KernelError::NoTasks);
        };

        let Some(nn_task) = self.tasks.get(&cur) else {
            self.current = None;
            return Err(KernelError::NoTasks);
        };

        nn_task.write().state = TaskState::Blocked { reason };
        Ok(cur)
    }

    #[allow(dead_code)]
    pub fn wake_task(&mut self, task_id: TaskId) -> Result<(), KernelError> {
        let Some(nn_task) = self.tasks.get(&task_id) else {
            return Err(KernelError::NoTasks);
        };

        let priority = {
            let mut task = nn_task.write();
            task.state = TaskState::Runnable;
            task.priority
        };

        self.queue_ready(task_id, priority);
        Ok(())
    }

    pub fn wake_task_with_return_value(
        &mut self,
        task_id: TaskId,
        return_value: u32,
    ) -> Result<(), KernelError> {
        let Some(nn_task) = self.tasks.get(&task_id) else {
            return Err(KernelError::NoTasks);
        };

        let priority = {
            let mut task = nn_task.write();
            task.registers.eax = return_value;
            task.state = TaskState::Runnable;
            task.priority
        };

        self.queue_ready(task_id, priority);
        Ok(())
    }

    pub fn exec_current(&mut self, process: Arc<Process>) -> Result<(), KernelError> {
        let Some(cur) = self.current else {
            return Err(KernelError::NoTasks);
        };

        let Some(nn_task) = self.tasks.get(&cur) else {
            self.current = None;
            return Err(KernelError::NoTasks);
        };

        let mut task = nn_task.write();
        task.registers = Task::entry_registers(&process);
        task.process = process;
        task.state = TaskState::Runnable;
        Ok(())
    }

    fn queue_ready(&mut self, task_id: TaskId, priority: usize) {
        let priority = priority.min(MAX_PRIORITY - 1);
        if !self.ready[priority].contains(&task_id) {
            self.ready[priority].push_back(task_id);
        }
    }

    fn find_runnable_task(&self) -> Option<(TaskId, Arc<Process>)> {
        self.tasks.iter().find_map(|(task_id, nn_task)| {
            let task = nn_task.read();
            task.is_runnable()
                .then(|| (*task_id, Arc::clone(&task.process)))
        })
    }

    fn has_waiting_tasks(&self) -> bool {
        self.tasks
            .values()
            .any(|nn_task| nn_task.read().is_waiting())
    }
}
