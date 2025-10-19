use crate::interrupts::{interrupt_frame::InterruptFrame, syscall::SyscallId};
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::RwLock;

const MAX_SYSCALLS: usize = 256;

pub type SyscallHandler = fn(frame: &InterruptFrame) -> u32;

lazy_static! {
    static ref SYSCALL_TABLE: Arc<RwLock<[Option<SyscallHandler>; MAX_SYSCALLS]>> =
        Arc::new(RwLock::new([None; MAX_SYSCALLS]));
}

#[inline]
pub fn syscall_register(id: SyscallId, handler: SyscallHandler) {
    let mut table = SYSCALL_TABLE.write();
    table[id as usize] = Some(handler);
}

#[inline]
pub fn syscall_get_handler(id: SyscallId) -> Option<SyscallHandler> {
    let table = SYSCALL_TABLE.read();
    table[id as usize]
}
