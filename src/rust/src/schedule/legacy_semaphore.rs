use super::{semaphore::Semaphore, task_manager::TaskManager};
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;
const EINVAL: i32 = 22;
const ESRCH: i32 = 3;
lazy_static! {
    pub static ref SEMAPHORES: Mutex<SemaphoreTable> = Mutex::new(SemaphoreTable::new());
}

pub struct SemaphoreTable {
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

    pub fn create(&mut self, count: isize) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1).max(1);
        self.semaphores.insert(id, Semaphore::new(id, count));
        id
    }

    pub fn wait(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<bool, i32> {
        self.semaphores
            .get_mut(&id)
            .ok_or(EINVAL)?
            .wait(task_manager)
            .map_err(|_| ESRCH)
    }

    pub fn signal(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<(), i32> {
        self.semaphores
            .get_mut(&id)
            .ok_or(EINVAL)
            .map(|semaphore| semaphore.signal(task_manager))
    }

    pub fn close(&mut self, id: usize, task_manager: &mut TaskManager) -> Result<(), i32> {
        self.semaphores
            .remove(&id)
            .ok_or(EINVAL)
            .map(|mut semaphore| semaphore.close(task_manager, (-EINVAL) as u32))
    }
}
