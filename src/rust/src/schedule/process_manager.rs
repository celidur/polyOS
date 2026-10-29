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

use super::process::{
    Process, ProcessArguments, ProcessId, SIG_DFL, SIG_IGN, SIGCHLD, SIGCONT, SIGKILL, SIGSTOP,
};

pub enum SignalEffect {
    Ignored,
    Delivered,
    Terminated,
}

pub struct ProcessManager {
    table: BTreeMap<ProcessId, Arc<Process>>,
    zombies: BTreeMap<ProcessId, ZombieProcess>,
    child_events: BTreeMap<ProcessId, VecDeque<(i32, u32)>>,
}

#[derive(Clone, Copy)]
struct ZombieProcess {
    parent_pid: Option<ProcessId>,
    status: i32,
    pgid: u32,
    uid: u32,
}

impl ProcessManager {
    pub fn setpgid(&mut self, caller: u32, pid: u32, group: u32) -> Result<(), i32> {
        let caller = self.table.get(&caller).ok_or(3)?.clone();
        let target = self.table.get(&pid).ok_or(3)?.clone();
        if pid != caller.pid && target.parent_pid() != Some(caller.pid) {
            return Err(3);
        }
        if pid != caller.pid && target.control.lock().execed {
            return Err(13);
        }
        let sid = caller.control.lock().sid;
        if target.control.lock().sid != sid || target.control.lock().sid == pid {
            return Err(1);
        }
        if group != pid
            && !self.table.values().any(|p| {
                let c = p.control.lock();
                c.pgid == group && c.sid == sid
            })
        {
            return Err(1);
        }
        target.control.lock().pgid = group;
        Ok(())
    }
    pub fn setsid(&mut self, pid: u32) -> Result<u32, i32> {
        if self.table.values().any(|p| p.control.lock().pgid == pid) {
            return Err(1);
        }
        let p = self.table.get(&pid).ok_or(3)?;
        let mut c = p.control.lock();
        c.sid = pid;
        c.pgid = pid;
        Ok(pid)
    }
    pub fn signal_group(&mut self, group: u32, signal: u32) {
        let ids: Vec<_> = self
            .table
            .iter()
            .filter_map(|(&id, p)| (p.control.lock().pgid == group).then_some(id))
            .collect();
        for id in ids {
            let _ = self.signal(id, signal);
        }
    }
    pub fn group_session(&self, group: u32) -> Option<u32> {
        self.table.values().find_map(|p| {
            let c = p.control.lock();
            (c.pgid == group).then_some(c.sid)
        })
    }
    pub fn send_signal(&mut self, caller: u32, selector: i32, signal: u32) -> Result<bool, i32> {
        let sender = self.table.get(&caller).ok_or(3)?.clone();
        let credentials = sender.control.lock().credentials.clone();
        let pgid = sender.control.lock().pgid;
        let sid = sender.control.lock().sid;
        let ids: Vec<_> = self
            .table
            .iter()
            .filter_map(|(&pid, p)| {
                let group = p.control.lock().pgid;
                let matches = match selector {
                    n if n > 0 => pid == n as u32,
                    0 => group == pgid,
                    -1 => pid != super::INIT_PID && pid != caller,
                    n => n.checked_neg().is_some_and(|n| group == n as u32),
                };
                matches.then_some(pid)
            })
            .collect();
        if ids.is_empty() {
            return Err(3);
        }
        let mut allowed = false;
        let mut affected = false;
        for id in ids {
            let Some(p) = self.table.get(&id) else {
                continue;
            };
            let target = p.control.lock();
            let permitted = (credentials.euid == 0
                || credentials.uid == target.credentials.uid
                || credentials.uid == target.credentials.saved_uid
                || credentials.euid == target.credentials.uid
                || credentials.euid == target.credentials.saved_uid
                || (signal == 18 && sid == target.sid));
            drop(target);
            if !permitted {
                continue;
            }
            allowed = true;
            let effect = self.signal(id, signal).map_err(|_| 3)?;
            if id == caller && matches!(effect, SignalEffect::Delivered | SignalEffect::Terminated)
            {
                affected = true;
            }
        }
        if allowed { Ok(affected) } else { Err(1) }
    }
    pub fn process_count(&self) -> usize {
        self.table.len()
    }

    pub fn new() -> Self {
        ProcessManager {
            table: BTreeMap::new(),
            zombies: BTreeMap::new(),
            child_events: BTreeMap::new(),
        }
    }
    pub fn spawn(
        &mut self,
        filename: &str,
        parent: Option<ProcessId>,
        arg: Option<ProcessArguments>,
    ) -> Result<ProcessId, KernelError> {
        let pid = super::allocate_id() as ProcessId;
        let process = Process::new(pid, parent, filename, arg)?;
        if let Some(parent) = parent.and_then(|id| self.table.get(&id)) {
            *process.control.lock() = parent.control.lock().clone();
            process.set_cwd(parent.cwd.lock().clone());
            process.set_umask(*parent.umask.lock());
            process.refresh_aux_ids();
        }
        let process = Arc::new(process);
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
        process.control.lock().execed = false;
        process.control.lock().auto_reap = false;
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

        let process = Process::new(pid, parent, filename, Some(args))?;
        *process.control.lock() = old_process.control.lock().clone();
        process.control.lock().execed = true;
        process.refresh_aux_ids();
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

    pub fn wait_event(
        &mut self,
        parent: u32,
        selector: i32,
        options: u32,
    ) -> Result<Option<(u32, i32, u32)>, i32> {
        let p = self.table.get(&parent).ok_or(10)?;
        let group = p.control.lock().pgid;
        let children = p.children.lock().clone();
        let mut eligible = false;
        for pid in children {
            let pgid = self
                .table
                .get(&pid)
                .map(|p| p.control.lock().pgid)
                .or_else(|| self.zombies.get(&pid).map(|z| z.pgid));
            let matches = match selector {
                n if n > 0 => pid == n as u32,
                -1 => true,
                0 => pgid == Some(group),
                n => n.checked_neg().is_some_and(|g| pgid == Some(g as u32)),
            };
            if !matches {
                continue;
            }
            eligible = true;
            if let Some(z) = self.zombies.get(&pid).copied() {
                self.reap_child(parent, pid);
                return Ok(Some((pid, z.status, z.uid)));
            }
            if let Some(events) = self.child_events.get_mut(&pid) {
                if let Some(index) = events.iter().position(|(s, _)| {
                    (*s == 0xffff && options & 8 != 0) || (*s & 255 == 0x7f && options & 2 != 0)
                }) {
                    let (status, uid) = events.remove(index).unwrap();
                    return Ok(Some((pid, status, uid)));
                }
            }
        }
        if !eligible { Err(10) } else { Ok(None) }
    }
    fn child_event(&mut self, process: &Process, status: i32) {
        let events = self.child_events.entry(process.pid).or_default();
        if events.len() >= 8 {
            events.pop_front();
        }
        events.push_back((status, process.uid()));
        if let Some(parent) = process.parent_pid() {
            KERNEL
                .with_task_manager(|tm| tm.wake_preserving(super::task::WaitReason::Child(parent)));
            let _ = self.signal(parent, SIGCHLD);
        }
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
        // PID 1 receives explicitly installed handlers, while default fatal/stop actions are ignored.
        if pid == super::INIT_PID && action.handler == SIG_DFL {
            return Ok(SignalEffect::Ignored);
        }
        if signal == SIGKILL {
            self.exit_with_wait_status(pid, encode_signal_status(signal));
            return Ok(SignalEffect::Terminated);
        }
        if signal == SIGCONT {
            self.child_event(&process, 0xffff);
            KERNEL.with_task_manager(|tm| tm.stop_process(pid, false));
            *process.pending_signals.lock() &= !((1 << 18) | (1 << 19) | (1 << 20) | (1 << 21));
        }
        if signal == SIGSTOP || (matches!(signal, 20 | 21 | 22) && action.handler == SIG_DFL) {
            self.child_event(&process, ((signal as i32) << 8) | 0x7f);
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

        // Adopted orphans are auto-reaped; init can still wait for children it created.
        let should_reap = process.parent_pid().is_none() || process.control.lock().auto_reap;
        if let Some(parent) = process.parent_pid() {
            KERNEL
                .with_task_manager(|tm| tm.wake_preserving(super::task::WaitReason::Child(parent)));
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
            let (pgid, uid) = {
                let c = process.control.lock();
                (c.pgid, c.credentials.uid)
            };
            self.zombies.insert(
                pid,
                ZombieProcess {
                    parent_pid: process.parent_pid(),
                    status: wait_status,
                    pgid,
                    uid,
                },
            );
            self.table.remove(&pid);
        }
    }

    fn reap_child(&mut self, parent_pid: ProcessId, pid: ProcessId) {
        if let Some(parent_process) = self.table.get(&parent_pid) {
            parent_process.children.lock().retain(|&x| x != pid);
        }
        self.child_events.remove(&pid);
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

        let new_parent = if pid != super::INIT_PID && self.table.contains_key(&super::INIT_PID) {
            Some(super::INIT_PID)
        } else {
            None
        };

        let auto_reap = new_parent.is_none() || new_parent == Some(super::INIT_PID);
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
            child.control.lock().auto_reap = auto_reap;
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
