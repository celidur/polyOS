use alloc::{collections::BTreeMap, sync::Arc};

use crate::{error::KernelError, kernel::KERNEL};

use super::process::{Process, ProcessArguments, ProcessId};

pub struct ProcessManager {
    table: BTreeMap<ProcessId, Arc<Process>>,
    id: ProcessId,
}
impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            table: BTreeMap::new(),
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

    pub fn get(&self, pid: ProcessId) -> Option<Arc<Process>> {
        self.table.get(&pid).cloned()
    }

    pub fn remove(&mut self, pid: ProcessId) {
        let process = self.table.remove(&pid);
        if let Some(process) = process {
            // if let Some(parent_pid) = process.parent {
            //     if let Some(parent_process) = self.table.get_mut(&parent_pid) {
            //         parent_process.children.lock().retain(|&x| x != pid);
            //     }
            // }
            process.cleanup();
            KERNEL.with_task_manager(|tm| {
                let task = process.tasks.read();
                if let Some(task) = task.as_ref() {
                    tm.remove(*task);
                }
            });
        }
    }
}
