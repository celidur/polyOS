pub(crate) mod loader;
pub mod process;
pub mod process_manager;
pub mod semaphore;
pub mod task;
pub mod task_manager;

pub mod thread;
use core::sync::atomic::{AtomicUsize, Ordering};
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
pub(super) fn allocate_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
