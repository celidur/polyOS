use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec::Vec,
};
use spin::RwLock;

use crate::{error::KernelError, interrupts::without_interrupts, schedule::task::user_registers};

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
    tick: u64,
}

impl TaskManager {
    pub fn stop_process(&mut self, pid: u32, stop: bool) -> bool {
        let mut changed = false;
        let mut wake = Vec::new();
        for (&id, t) in &self.tasks {
            let mut t = t.write();
            if t.process.pid == pid {
                changed |= t.stopped != stop;
                t.stopped = stop;
                t.signals.pending &= if stop {
                    !(1 << 17)
                } else {
                    !((1 << 18) | (1 << 19) | (1 << 20) | (1 << 21))
                };
                if !stop && t.state.is_runnable() {
                    wake.push(id);
                }
            }
        }
        for id in wake {
            let _ = self.wake_task(id);
        }
        changed
    }
    pub fn discard_pending_signals(&mut self, pid: u32, mask: u32) {
        for task in self.tasks.values() {
            let mut task = task.write();
            if task.process.pid == pid {
                task.signals.pending &= !mask;
            }
        }
    }
    pub fn can_deliver_signal(&self, pid: u32, signal: u32) -> bool {
        self.tasks.values().any(|task| {
            let task = task.read();
            task.process.pid == pid && task.signals.blocked & (1 << (signal - 1)) == 0
        })
    }
    pub fn notify_signal(&mut self, pid: u32, signal: u32) -> bool {
        let target = self.tasks.iter().find_map(|(&id, t)| {
            let t = t.read();
            (t.process.pid == pid && t.signals.blocked & (1 << (signal - 1)) == 0).then_some(id)
        });
        let Some(id) = target else {
            return false;
        };
        {
            let mut t = self.tasks.get(&id).unwrap().write();
            t.signals.pending |= 1 << (signal - 1);
            if !t.state.is_runnable() {
                if t.restarting {
                    t.registers.ip += 2;
                    t.restarting = false;
                }
                t.registers.eax = (-4i32) as u32;
                t.poll_deadline = None;
                if let Some((ptr, end)) = t.sleep_remaining.take() {
                    let ticks = end.saturating_sub(self.tick);
                    let hz = crate::constant::TIMER_HZ as u64;
                    let rem = [
                        (ticks / hz) as u32,
                        ((ticks % hz) * 1_000_000_000 / hz) as u32,
                    ];
                    if crate::memory::user::validate_range(&t.process, ptr, 8, true).is_ok() {
                        let _ =
                            crate::memory::user::write_value(&t.process.page_directory, ptr, &rem);
                    }
                }
            }
        }
        let _ = self.wake_task(id);
        true
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn new() -> Self {
        TaskManager {
            tasks: BTreeMap::new(),
            ready: [VecDeque::new()],
            current: None,
            tick: 0,
        }
    }

    pub fn spawn(&mut self, process: Arc<Process>) -> Result<TaskId, KernelError> {
        let priority = 0;
        let id = process.pid as usize;
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
        let id = process.pid as usize;
        let mut task = Task::from_registers(id, process, priority, registers);
        if let Some(current) = self.get_current() {
            task.fpu = current.read().fpu;
        }
        task.name = self
            .get_current()
            .map(|t| t.read().name)
            .unwrap_or(task.name);
        task.signals = self
            .get_current()
            .map(|t| t.read().signals.fork_context())
            .unwrap_or_default();
        self.tasks.insert(id, RwLock::new(task));
        self.queue_ready(id, priority);
        Ok(id)
    }

    pub fn spawn_thread(
        &mut self,
        process: Arc<Process>,
        registers: Registers,
        priority: usize,
        clear_child_tid: u32,
    ) -> Result<TaskId, KernelError> {
        let id = super::allocate_id();
        let mut task = Task::from_registers(id, process, priority, registers);
        if let Some(current) = self.get_current() {
            task.fpu = current.read().fpu;
        }
        task.name = self
            .get_current()
            .map(|t| t.read().name)
            .unwrap_or(task.name);
        task.clear_child_tid = clear_child_tid;
        task.signals.blocked = self
            .get_current()
            .map(|t| t.read().signals.blocked)
            .unwrap_or(0);
        self.tasks.insert(id, RwLock::new(task));
        self.queue_ready(id, priority);
        Ok(id)
    }

    pub fn process_task_count(&self, pid: u32) -> usize {
        self.tasks
            .values()
            .filter(|task| task.read().process.pid == pid)
            .count()
    }
    pub fn inspection(&self, pid: u32) -> (usize, u64, char, [u8; 16]) {
        let mut count = 0; let mut ticks = 0; let mut state = 'S'; let mut name = [0; 16];
        for task in self.tasks.values() {
            let task = task.read();
            if task.process.pid != pid { continue; }
            if count == 0 { name = task.name; }
            count += 1; ticks += task.user_ticks;
            if task.stopped { state = 'T'; } else if task.is_runnable() { state = 'R'; }
        }
        (count, ticks, state, name)
    }

    pub fn remove_process_tasks(&mut self, pid: u32, keep: Option<TaskId>) {
        let ids: Vec<_> = self
            .tasks
            .iter()
            .filter_map(|(&id, task)| {
                (task.read().process.pid == pid && Some(id) != keep).then_some(id)
            })
            .collect();
        for id in ids {
            self.remove(id);
        }
    }

    pub fn wake_preserving(&mut self, reason: WaitReason) {
        let ids: Vec<_> = self
            .tasks
            .iter()
            .filter_map(|(&id, t)| {
                matches!(t.read().state,TaskState::Blocked{reason:r} if r==reason).then_some(id)
            })
            .collect();
        for id in ids {
            let _ = self.wake_task(id);
        }
    }
    pub fn wake_reason(&mut self, reason: WaitReason, limit: usize) -> usize {
        let ids: Vec<_> = self
            .tasks
            .iter()
            .filter_map(|(&id, task)| {
                matches!(task.read().state, TaskState::Blocked {reason:r} | TaskState::BlockedUntil {reason:r,..} if r==reason).then_some(id)
            })
            .take(limit)
            .collect();
        let count = ids.len();
        for id in ids {
            let _ = self.wake_task_with_return_value(id, 0);
        }
        count
    }

    pub fn task_page(&self) -> Result<(), KernelError> {
        if let Some(cur) = self.current
            && let Some(nn_cur) = self.tasks.get(&cur)
        {
            let process = &nn_cur.read().process;
            without_interrupts(|| {
                user_registers();
                process.page_directory.switch();
            });
            return Ok(());
        }
        Err(KernelError::NoTasks)
    }

    pub fn schedule(&mut self) -> ScheduleOutcome {
        crate::metrics::scheduled();
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

                if !nn_next.read().is_runnable() {
                    continue;
                }

                self.current = Some(next_id);
                return ScheduleOutcome::Switched;
            }
        }

        self.current = None;

        if let Some((task_id, _process)) = self.find_runnable_task() {
            self.current = Some(task_id);
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
        if let Some(task) = self.get_current() { task.write().user_ticks += 1; }
        self.tick = self.tick.wrapping_add(1);

        let now = self.tick;
        let mut wake: Vec<(TaskId, usize)> = Vec::new();
        for (task_id, nn_task) in &self.tasks {
            let mut task = nn_task.write();
            if let TaskState::BlockedUntil { wake_tick, .. } = task.state
                && now >= wake_tick
            {
                task.registers.eax = (-110i32) as u32;
                task.state = TaskState::Runnable;
                task.sleep_remaining = None;
                wake.push((*task_id, task.priority));
            }
            if let TaskState::Sleeping { wake_tick } = task.state
                && now >= wake_tick
            {
                task.state = TaskState::Runnable;
                task.sleep_remaining = None;
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

    pub fn block_current_and_restart_syscall(
        &mut self,
        reason: WaitReason,
    ) -> Result<TaskId, KernelError> {
        let Some(cur) = self.current else {
            return Err(KernelError::NoTasks);
        };

        let Some(nn_task) = self.tasks.get(&cur) else {
            self.current = None;
            return Err(KernelError::NoTasks);
        };

        let mut task = nn_task.write();
        task.registers.ip = task.registers.ip.saturating_sub(2);
        task.restarting = true;
        task.state = TaskState::Blocked { reason };
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

        let priority = nn_task.read().priority;
        let mask = nn_task.read().signals.blocked;
        self.remove(cur);
        let id = process.pid as usize;
        self.tasks
            .insert(id, RwLock::new(Task::new(id, process, priority)));
        self.tasks.get(&id).unwrap().write().signals.blocked = mask;
        self.current = Some(id);
        Ok(())
    }

    pub fn first_process_task(&self, pid: u32) -> Option<TaskId> {
        self.tasks
            .iter()
            .find_map(|(&id, task)| (task.read().process.pid == pid).then_some(id))
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
