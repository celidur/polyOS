use crate::interrupts::{interrupt_frame::InterruptFrame, syscall::SyscallId};
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::RwLock;

const MAX_SYSCALLS: usize = 1024;

/// Typed adapters isolate the machine ABI from subsystem syscall handlers.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum SyscallHandler {
    Args0(fn() -> u32),
    Args1(fn(u32) -> u32),
    Args2(fn(u32, u32) -> u32),
    Args3(fn(u32, u32, u32) -> u32),
    Args4(fn(u32, u32, u32, u32) -> u32),
    Args5(fn(u32, u32, u32, u32, u32) -> u32),
    Args6(fn(u32, u32, u32, u32, u32, u32) -> u32),
}

impl SyscallHandler {
    pub fn call(&self, frame: &InterruptFrame) -> u32 {
        match *self {
            Self::Args6(f) => f(
                frame.syscall_arg1(), frame.syscall_arg2(), frame.syscall_arg3(), frame.syscall_arg4(), frame.syscall_arg5(), frame.syscall_arg6(),
            ),
            Self::Args0(f) => f(),
            Self::Args1(f) => f(frame.syscall_arg1()),
            Self::Args2(f) => f(frame.syscall_arg1(), frame.syscall_arg2()),
            Self::Args3(f) => f(
                frame.syscall_arg1(),
                frame.syscall_arg2(),
                frame.syscall_arg3(),
            ),
            Self::Args4(f) => f(
                frame.syscall_arg1(),
                frame.syscall_arg2(),
                frame.syscall_arg3(),
                frame.syscall_arg4(),
            ),
            Self::Args5(f) => f(
                frame.syscall_arg1(),
                frame.syscall_arg2(),
                frame.syscall_arg3(),
                frame.syscall_arg4(),
                frame.syscall_arg5(),
            ),
        }
    }
}

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
