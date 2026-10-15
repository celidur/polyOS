//! ABI validation only; task ownership and wait queues belong to the scheduler.
use super::{abi, user};
use crate::{
    kernel::KERNEL,
    schedule::{
        process_manager::process_terminate,
        task::{WaitReason, task_current_set_return_value, task_next},
        thread,
    },
};
const SHARED_THREAD: u32 = 0x100 | 0x200 | 0x400 | 0x800 | 0x10000;
const PARENT_SETTID: u32 = 0x100000;
const CHILD_CLEARTID: u32 = 0x200000;

pub fn syscall_gettid() -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|t| t.read().id as u32)
            .unwrap_or_else(|| abi::errno(abi::ESRCH))
    })
}

pub fn syscall_clone(flags: u32, stack: u32, parent_tid: u32, tls: u32, child_tid: u32) -> u32 {
    if flags & SHARED_THREAD != SHARED_THREAD
        || flags & !(SHARED_THREAD | PARENT_SETTID | CHILD_CLEARTID) != 0
        || tls != 0
    {
        return abi::errno(abi::EINVAL);
    }
    KERNEL.with_task_manager(|tm| {
        let Some((process, mut registers, priority)) = tm.get_current().map(|t| {
            let t = t.read();
            (t.process.clone(), t.registers, t.priority)
        }) else {
            return abi::errno(abi::ESRCH);
        };
        if stack == 0
            || user::validate_range(&process, stack, 8, true).is_err()
            || (flags & PARENT_SETTID != 0
                && user::validate_range(&process, parent_tid, 4, true).is_err())
            || (flags & CHILD_CLEARTID != 0
                && user::validate_range(&process, child_tid, 4, true).is_err())
        {
            return abi::errno(abi::EFAULT);
        }
        registers.esp = stack;
        registers.eax = 0;
        let clear = if flags & CHILD_CLEARTID != 0 {
            child_tid
        } else {
            0
        };
        let tid = match tm.spawn_thread(process.clone(), registers, priority, clear) {
            Ok(id) => id as u32,
            Err(_) => return abi::errno(abi::EAGAIN),
        };
        if flags & PARENT_SETTID != 0
            && user::write_value(&process.page_directory, parent_tid, &tid).is_err()
        {
            tm.remove(tid as usize);
            return abi::errno(abi::EFAULT);
        }
        tid
    })
}

pub fn syscall_exit(code: u32) -> u32 {
    thread::exit_current(code as i32)
}
pub fn syscall_exit_group(code: u32) -> u32 {
    process_terminate(code as i32);
    task_next()
}
pub fn syscall_sched_yield() -> u32 {
    task_current_set_return_value(0);
    task_next()
}

pub fn syscall_futex(address: u32, op: u32, value: u32, timeout: u32) -> u32 {
    if op & !128 == 1 && value > i32::MAX as u32 {
        return abi::errno(abi::EINVAL);
    }
    if address & 3 != 0 {
        return abi::errno(abi::EINVAL);
    }
    if op & !128 > 1 {
        return abi::errno(abi::ENOSYS);
    }
    if op & !128 == 0 && timeout != 0 {
        return abi::errno(abi::ENOTSUP);
    }
    let result = KERNEL.with_task_manager(|tm| {
        let Some(process) = tm.get_current().map(|t| t.read().process.clone()) else {
            return Err(abi::ESRCH);
        };
        if user::validate_range(&process, address, 4, false).is_err() {
            return Err(abi::EFAULT);
        }
        let reason = WaitReason::Futex {
            pid: process.pid,
            address,
        };
        if op & !128 == 1 {
            return Ok(Some(tm.wake_reason(reason, value as usize) as u32));
        }
        let mut actual = 0_u32;
        user::copy_from_user(
            &process.page_directory,
            address,
            &mut actual as *mut u32 as *mut u8,
            4,
        )
        .map_err(|_| abi::EFAULT)?;
        if actual != value {
            return Err(abi::EAGAIN);
        }
        tm.block_current(reason).map_err(|_| abi::ESRCH)?;
        Ok(None)
    });
    match result {
        Ok(Some(result)) => result,
        Ok(None) => {
            task_current_set_return_value(0);
            task_next()
        }
        Err(error) => abi::errno(error),
    }
}
