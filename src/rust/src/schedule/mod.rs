pub(crate) mod loader;
pub mod process;
pub mod process_manager;
pub mod semaphore;
pub mod task;
pub mod task_manager;

pub mod thread;
use core::sync::atomic::{AtomicUsize, Ordering};
pub const INIT_PID: u32 = 1;
static NEXT_ID: AtomicUsize = AtomicUsize::new(INIT_PID as usize);
pub(super) fn allocate_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub mod ipc;

pub mod legacy_semaphore;

pub mod signal;

pub mod control;
