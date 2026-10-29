use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec::Vec,
};

use crate::{
    error::KernelError,
    kernel::KERNEL,
    schedule::{
        task::{Registers, TaskId},
        task_manager::TaskManager,
    },
};

use super::{
    process::{
        Process, ProcessArguments, ProcessId, SIG_DFL, SIG_IGN, SIGCHLD, SIGCONT, SIGKILL, SIGSTOP,
    },
    task::TaskState,
};

pub enum SignalEffect {
    Ignored,
    Delivered,
    Terminated,
}

pub struct ProcessManager {
    table: BTreeMap<ProcessId, Arc<Process>>,
    zombies: BTreeMap<ProcessId, ZombieProcess>,
    exit_waiters: BTreeMap<ProcessId, VecDeque<ExitWaiter>>,
}

#[derive(Clone, Copy)]
struct ExitWaiter {
    task_id: TaskId,
    status_ptr: u32,
    return_value: u32,
}

#[derive(Clone, Copy)]
struct ZombieProcess {
    parent_pid: Option<ProcessId>,
    status: i32,
}

impl ProcessManager {
    pub fn process_count(&self) -> usize {
        self.table.len()
    }

    pub fn new() -> Self {
        ProcessManager {
            table: BTreeMap::new(),
            zombies: BTreeMap::new(),
            exit_waiters: BTreeMap::new(),
        }
    }
    pub fn spawn(
        &mut self,
        filename: &str,
        parent: Option<ProcessId>,
        arg: Option<ProcessArguments>,
    ) -> Result<ProcessId, KernelError> {
        let pid = super::allocate_id() as ProcessId;
        let process = Arc::new(Process::new(pid, parent, filename, arg)?);
        self.table.insert(pid, process.clone());
        if let Some(parent_pid) = parent
            && let Some(parent_process) = self.table.get(&parent_pid)
        {
            parent_process.children.lock().push(pid);
        }

        let res = KERNEL.with_task_manager(|tm| tm.spawn(process.clone()))?;

        process.main_task.write().replace(res);

        Ok(pid)
    }

    pub fn fork(
        &mut self,
        parent: Arc<Process>,
        child_registers: Registers,
        priority: usize,
    ) -> Result<ProcessId, KernelError> {
        let pid = super::allocate_id() as ProcessId;

        let process = Arc::new(Process::fork_from(pid, &parent)?);
        self.table.insert(pid, process.clone());
        parent.children.lock().push(pid);

        let task_id: TaskId = KERNEL.with_task_manager(|tm| {
            tm.spawn_with_registers(process.clone(), child_registers, priority)
        })?;

        process.main_task.write().replace(task_id);

        Ok(pid)
    }

    pub fn get(&self, pid: ProcessId) -> Option<Arc<Process>> {
        self.table.get(&pid).cloned()
    }

    pub fn exec(
        &mut self,
        pid: ProcessId,
        filename: &str,
        args: ProcessArguments,
    ) -> Result<(), KernelError> {
        let old_process = self.table.get(&pid).cloned().ok_or(KernelError::NoTasks)?;
        let parent = old_process.parent_pid();
        let task_id = Some(pid as usize);
        let children = old_process.children.lock().clone();
        let cwd = old_process.cwd.lock().clone();
        let umask = *old_process.umask.lock();
        let signal_actions = old_process.signal_actions_for_exec();

        let mut process = Process::new(pid, parent, filename, Some(args))?;
        process.uid = old_process.uid;
        process.gid = old_process.gid;
        process.euid = old_process.euid;
        process.egid = old_process.egid;
        let fd_table = old_process.take_exec_fd_table();
        *process.fd_table.lock() = fd_table;
        process.set_cwd(cwd);
        process.set_umask(umask);
        process.replace_signal_actions(signal_actions);
        process.children.lock().extend(children);
        *process.main_task.write() = task_id;

        let process = Arc::new(process);
        self.table.insert(pid, process.clone());
        KERNEL.with_task_manager(|tm| {
            let current = tm.get_current().map(|task| task.read().id);
            tm.remove_process_tasks(pid, current);
        });
        old_process.cleanup();

        KERNEL.with_task_manager(|tm| tm.exec_current(process))
    }

    pub fn wait_for_exit(
        &mut self,
        parent_pid: ProcessId,
        pid: ProcessId,
        waiter: Option<(TaskId, u32, u32)>,
    ) -> Result<Option<i32>, KernelError> {
        let Some(parent_process) = self.table.get(&parent_pid) else {
            return Err(KernelError::NoTasks);
        };

        if !parent_process.children.lock().contains(&pid) {
            return Err(KernelError::NoTasks);
        }

        if let Some(zombie) = self.zombies.get(&pid).copied() {
            if zombie.parent_pid != Some(parent_pid) {
                return Err(KernelError::NoTasks);
            }

            self.reap_child(parent_pid, pid);
            return Ok(Some(zombie.status));
        }

        let Some(child) = self.table.get(&pid).cloned() else {
            return Err(KernelError::NoTasks);
        };

        if let Some(status) = child.zombie_status() {
            self.reap_child(parent_pid, pid);
            return Ok(Some(status));
        }

        if let Some((task_id, status_ptr, return_value)) = waiter {
            self.exit_waiters
                .entry(pid)
                .or_default()
                .push_back(ExitWaiter {
                    task_id,
                    status_ptr,
                    return_value,
                });
        }
        Ok(None)
    }

    pub fn exit(&mut self, pid: ProcessId, code: i32) {
        self.exit_with_wait_status(pid, encode_exit_status(code));
    }

    pub fn signal(&mut self, pid: ProcessId, signal: u32) -> Result<SignalEffect, KernelError> {
        let process = self.table.get(&pid).cloned().ok_or(KernelError::NoTasks)?;
        if signal == 0 {
            return Ok(SignalEffect::Ignored);
        }
        let action = process
            .get_signal_action(signal)
            .ok_or(KernelError::NoTasks)?;
        if signal == SIGKILL {
            self.exit_with_wait_status(pid, encode_signal_status(signal));
            return Ok(SignalEffect::Terminated);
        }
        if signal == SIGCONT {
            KERNEL.with_task_manager(|tm| tm.stop_process(pid, false));
            *process.pending_signals.lock() &= !((1 << 18) | (1 << 19) | (1 << 20) | (1 << 21));
        }
        if signal == SIGSTOP || (matches!(signal, 20 | 21 | 22) && action.handler == SIG_DFL) {
            KERNEL.with_task_manager(|tm| tm.stop_process(pid, true));
            *process.pending_signals.lock() &= !(1 << 17);
            return Ok(SignalEffect::Delivered);
        }
        if action.handler == SIG_IGN
            || (action.handler == SIG_DFL && matches!(signal, 17 | 18 | 23 | 28))
        {
            return Ok(SignalEffect::Ignored);
        }
        *process.pending_signals.lock() |= 1 << (signal - 1);
        let notified = KERNEL.with_task_manager(|tm| tm.notify_signal(pid, signal));
        if notified {
            *process.pending_signals.lock() &= !(1 << (signal - 1));
        }
        Ok(if notified {
            SignalEffect::Delivered
        } else {
            SignalEffect::Ignored
        })
    }
    pub fn terminate_signal(&mut self, pid: ProcessId, signal: u32) {
        self.exit_with_wait_status(pid, encode_signal_status(signal));
    }

    fn exit_with_wait_status(&mut self, pid: ProcessId, wait_status: i32) {
        let Some(process) = self.table.get(&pid).cloned() else {
            return;
        };

        self.reparent_children(pid);
        KERNEL.with_task_manager(|tm| tm.remove_process_tasks(pid, None));
        process.close_descriptors();
        process.cleanup();
        process.mark_zombie(wait_status);

        // pid 0 is the init-like reaper, so orphaned children do not linger as zombies.
        let mut should_reap = matches!(process.parent_pid(), None | Some(0));
        if let Some(mut waiters) = self.exit_waiters.remove(&pid) {
            let mut consumed = false;
            KERNEL.with_task_manager(|tm| {
                while let Some(waiter) = waiters.pop_front() {
                    let waiting = tm.get(waiter.task_id).is_some_and(|t| {
                        t.read().state
                            == TaskState::Blocked {
                                reason: super::task::WaitReason::Process(pid as usize),
                            }
                    });
                    if !waiting {
                        continue;
                    }
                    if consumed {
                        let _ = tm.wake_task_with_return_value(waiter.task_id, (-10i32) as u32);
                        continue;
                    }
                    consumed = true;
                    if waiter.status_ptr != 0
                        && let Some(task) = tm.get(waiter.task_id)
                    {
                        let task = task.read();
                        if crate::memory::user::validate_range(
                            &task.process,
                            waiter.status_ptr,
                            4,
                            true,
                        )
                        .is_ok()
                        {
                            let _ = crate::memory::user::write_value(
                                &task.process.page_directory,
                                waiter.status_ptr,
                                &wait_status,
                            );
                        }
                    }

                    let _ = tm.wake_task_with_return_value(waiter.task_id, waiter.return_value);
                }
            });
            should_reap |= consumed;
        }

        if let Some(parent_pid) = process.parent_pid() {
            let _ = self.signal(parent_pid, SIGCHLD);
        }

        if should_reap {
            if let Some(parent_pid) = process.parent_pid() {
                self.reap_child(parent_pid, pid);
            } else {
                self.table.remove(&pid);
            }
        } else {
            self.zombies.insert(
                pid,
                ZombieProcess {
                    parent_pid: process.parent_pid(),
                    status: wait_status,
                },
            );
            self.table.remove(&pid);
        }
    }

    fn reap_child(&mut self, parent_pid: ProcessId, pid: ProcessId) {
        if let Some(parent_process) = self.table.get(&parent_pid) {
            parent_process.children.lock().retain(|&x| x != pid);
        }
        self.exit_waiters.remove(&pid);
        self.zombies.remove(&pid);
        self.table.remove(&pid);
    }

    fn reparent_children(&mut self, pid: ProcessId) {
        let Some(process) = self.table.get(&pid).cloned() else {
            return;
        };

        let children = {
            let mut children = process.children.lock();
            core::mem::take(&mut *children)
        };

        let new_parent = if pid != 0 && self.table.contains_key(&0) {
            Some(0)
        } else {
            None
        };

        let auto_reap = new_parent.is_none() || new_parent == Some(0);
        let mut reaped_zombies = Vec::new();

        for child_pid in children {
            let Some(child) = self.table.get(&child_pid).cloned() else {
                if let Some(zombie) = self.zombies.get_mut(&child_pid) {
                    if auto_reap {
                        reaped_zombies.push(child_pid);
                        continue;
                    }

                    zombie.parent_pid = new_parent;
                    if let Some(new_parent) = new_parent
                        && let Some(parent) = self.table.get(&new_parent)
                    {
                        let mut siblings = parent.children.lock();
                        if !siblings.contains(&child_pid) {
                            siblings.push(child_pid);
                        }
                    }
                }
                continue;
            };

            child.set_parent(new_parent);
            if let Some(new_parent) = new_parent {
                if auto_reap && child.zombie_status().is_some() {
                    self.table.remove(&child_pid);
                    continue;
                }

                if let Some(parent) = self.table.get(&new_parent) {
                    let mut siblings = parent.children.lock();
                    if !siblings.contains(&child_pid) {
                        siblings.push(child_pid);
                    }
                }
            } else if child.zombie_status().is_some() {
                self.table.remove(&child_pid);
            }
        }

        for child_pid in reaped_zombies {
            self.zombies.remove(&child_pid);
        }
    }
}

pub fn process_terminate(code: i32) {
    let pid = match KERNEL.with_task_manager(|tm: &mut TaskManager| {
        tm.get_current()
            .map(|current_task| current_task.read().process.pid)
    }) {
        Some(pid) => pid,
        None => return,
    };

    KERNEL.with_process_manager(|pm: &mut ProcessManager| {
        pm.exit(pid, code);
    });
}

fn encode_exit_status(code: i32) -> i32 {
    (code & 0xff) << 8
}

fn encode_signal_status(signal: u32) -> i32 {
    (signal & 0x7f) as i32
}
