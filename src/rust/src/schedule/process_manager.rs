use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use crate::{
    error::KernelError,
    kernel::KERNEL,
    schedule::{
        task::{Registers, TaskId, copy_string_to_task},
        task_manager::TaskManager,
    },
};

use super::process::{Process, ProcessArguments, ProcessId};

pub struct ProcessManager {
    table: BTreeMap<ProcessId, Arc<Process>>,
    exit_status: BTreeMap<ProcessId, i32>,
    exit_waiters: BTreeMap<ProcessId, VecDeque<ExitWaiter>>,
    id: ProcessId,
}

#[derive(Clone, Copy)]
struct ExitWaiter {
    task_id: TaskId,
    status_ptr: u32,
    return_value: u32,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            table: BTreeMap::new(),
            exit_status: BTreeMap::new(),
            exit_waiters: BTreeMap::new(),
            id: 0,
        }
    }
    pub fn spawn(
        &mut self,
        filename: &str,
        parent: Option<ProcessId>,
        arg: Option<ProcessArguments>,
    ) -> Result<ProcessId, KernelError> {
        let pid = self.id;
        self.id += 1;
        let process = Arc::new(Process::new(pid, parent, filename, arg)?);
        self.table.insert(pid, process.clone());
        if let Some(parent_pid) = parent
            && let Some(parent_process) = self.table.get_mut(&parent_pid)
        {
            parent_process.children.lock().push(pid);
        }

        let res = KERNEL.with_task_manager(|tm| tm.spawn(process.clone()))?;

        process.tasks.write().replace(res);

        Ok(pid)
    }

    pub fn fork(
        &mut self,
        parent: Arc<Process>,
        child_registers: Registers,
        priority: usize,
    ) -> Result<ProcessId, KernelError> {
        let pid = self.id;
        self.id += 1;

        let process = Arc::new(Process::fork_from(pid, &parent)?);
        self.table.insert(pid, process.clone());
        parent.children.lock().push(pid);

        let task_id: TaskId = KERNEL.with_task_manager(|tm| {
            tm.spawn_with_registers(process.clone(), child_registers, priority)
        })?;

        process.tasks.write().replace(task_id);

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
        let parent = old_process.parent;
        let task_id = *old_process.tasks.read();
        let children = old_process.children.lock().clone();
        let fd_table = {
            let mut old_fd_table = old_process.fd_table.lock();
            core::mem::take(&mut *old_fd_table)
        };

        let process = Process::new(pid, parent, filename, Some(args))?;
        *process.fd_table.lock() = fd_table;
        process.children.lock().extend(children);
        *process.tasks.write() = task_id;

        let process = Arc::new(process);
        self.table.insert(pid, process.clone());
        old_process.cleanup();

        KERNEL.with_task_manager(|tm| tm.exec_current(process))
    }

    pub fn wait_for_exit(
        &mut self,
        parent_pid: ProcessId,
        pid: ProcessId,
        waiter: TaskId,
        status_ptr: u32,
        return_value: u32,
    ) -> Result<Option<i32>, KernelError> {
        if let Some(status) = self.exit_status.remove(&pid) {
            if let Some(parent_process) = self.table.get_mut(&parent_pid) {
                parent_process.children.lock().retain(|&x| x != pid);
            }
            return Ok(Some(status));
        }

        if !self.table.contains_key(&pid) {
            return Err(KernelError::NoTasks);
        }

        self.exit_waiters
            .entry(pid)
            .or_default()
            .push_back(ExitWaiter {
                task_id: waiter,
                status_ptr,
                return_value,
            });
        Ok(None)
    }

    pub fn exit(&mut self, pid: ProcessId, code: i32) {
        let has_waiters = self.exit_waiters.contains_key(&pid);
        let process = self.table.remove(&pid);
        if let Some(process) = process {
            process.close_descriptors();
            if has_waiters
                && let Some(parent_pid) = process.parent
                && let Some(parent_process) = self.table.get_mut(&parent_pid)
            {
                parent_process.children.lock().retain(|&x| x != pid);
            }
            process.cleanup();
            KERNEL.with_task_manager(|tm| {
                let task = process.tasks.read();
                if let Some(task) = task.as_ref() {
                    tm.remove(*task);
                }
            });
        }

        if let Some(mut waiters) = self.exit_waiters.remove(&pid) {
            KERNEL.with_task_manager(|tm| {
                while let Some(waiter) = waiters.pop_front() {
                    if waiter.status_ptr != 0
                        && let Some(task) = tm.get(waiter.task_id)
                    {
                        let task = task.read();
                        let _ = copy_string_to_task(
                            &task.process.page_directory,
                            &code as *const i32 as u32,
                            waiter.status_ptr,
                            core::mem::size_of::<i32>() as u32,
                        );
                    }

                    let _ = tm.wake_task_with_return_value(waiter.task_id, waiter.return_value);
                }
            });
        } else {
            self.exit_status.insert(pid, code);
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
