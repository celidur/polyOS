use crate::{
    constant::{HEAP_SIZE_BYTES, TIMER_HZ},
    kernel::KERNEL,
    memory,
};
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub ticks: u64,
    pub total: u32,
    pub free: u32,
    pub processes: u16,
    pub tasks: usize,
}
pub fn snapshot() -> Snapshot {
    let (ticks, tasks) = KERNEL.with_task_manager(|tm| (tm.get_tick(), tm.task_count()));
    let processes = KERNEL.with_process_manager(|pm| pm.process_count());
    Snapshot {
        ticks,
        total: HEAP_SIZE_BYTES as u32,
        free: HEAP_SIZE_BYTES.saturating_sub(memory::allocated_bytes()) as u32,
        processes: processes.min(u16::MAX as usize) as u16,
        tasks,
    }
}
impl Snapshot {
    pub fn uptime(&self) -> u32 {
        (self.ticks / TIMER_HZ as u64) as u32
    }
}
