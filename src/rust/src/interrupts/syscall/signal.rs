use crate::{
    kernel::KERNEL,
    schedule::{
        process::{SIGKILL, SIGNAL_FRAME_MAGIC, SIGSTOP, SignalAction, SignalFrame, valid_signal},
        task::{task_current_set_return_value, task_next},
    },
};

use super::{abi, user};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct UserSignalAction {
    handler: u32,
    flags: u32,
    restorer: u32,
    mask: u32,
}

impl From<SignalAction> for UserSignalAction {
    fn from(action: SignalAction) -> Self {
        Self {
            handler: action.handler,
            flags: action.flags,
            restorer: action.restorer,
            mask: action.mask,
        }
    }
}

impl From<UserSignalAction> for SignalAction {
    fn from(action: UserSignalAction) -> Self {
        Self {
            handler: action.handler,
            flags: action.flags,
            restorer: action.restorer,
            mask: action.mask,
        }
    }
}

pub fn syscall_kill(selector: u32, signal: u32) -> u32 {
    if signal != 0 && !valid_signal(signal) {
        return abi::errno(abi::EINVAL);
    }
    let Some(pid) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.pid))
    else {
        return abi::errno(abi::ESRCH);
    };
    task_current_set_return_value(0);
    match KERNEL.with_process_manager(|pm| pm.send_signal(pid, selector as i32, signal)) {
        Ok(true) => task_next(),
        Ok(false) => 0,
        Err(e) => abi::errno(e),
    }
}

pub fn syscall_sigaction(signum: u32, act_ptr: u32, oldact_ptr: u32) -> u32 {
    let args = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), signum, act_ptr, oldact_ptr))
    });

    let Some((process, signal, act_ptr, oldact_ptr)) = args else {
        return abi::errno(abi::ESRCH);
    };

    if !valid_signal(signal) {
        return abi::errno(abi::EINVAL);
    }
    if act_ptr != 0 && (signal == SIGKILL || signal == SIGSTOP) {
        return abi::errno(abi::EINVAL);
    }

    let Some(old_action) = process.get_signal_action(signal) else {
        return abi::errno(abi::EINVAL);
    };

    let mut user_action = UserSignalAction::default();
    if act_ptr != 0
        && user::copy_from_user(
            &process.page_directory,
            act_ptr,
            &mut user_action as *mut UserSignalAction as *mut u8,
            core::mem::size_of::<UserSignalAction>() as u32,
        )
        .is_err()
    {
        return abi::errno(abi::EFAULT);
    }
    if oldact_ptr != 0 {
        let old: UserSignalAction = old_action.into();
        if user::write_value(&process.page_directory, oldact_ptr, &old).is_err() {
            return abi::errno(abi::EFAULT);
        }
    }
    if act_ptr != 0 {
        // SA_ONSTACK, SA_NODEFER and SA_RESETHAND. Restart semantics are not silently promised.
        if user_action.flags & !(0x08000000 | 0x40000000 | 0x80000000) != 0 {
            return abi::errno(abi::ENOTSUP);
        }
        if user_action.handler > 1 && user_action.restorer == 0 {
            return abi::errno(abi::EINVAL);
        }

        let _ = process.set_signal_action(signal, user_action.into());
    }

    0
}

pub fn syscall_sigreturn(frame_ptr: u32) -> u32 {
    let Some((process, frame_ptr)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), frame_ptr))
    }) else {
        return abi::errno(abi::ESRCH);
    };

    if frame_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let mut signal_frame = SignalFrame {
        magic: 0,
        registers: Default::default(),
    };
    if user::copy_from_user(
        &process.page_directory,
        frame_ptr,
        &mut signal_frame as *mut SignalFrame as *mut u8,
        core::mem::size_of::<SignalFrame>() as u32,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    if signal_frame.magic != SIGNAL_FRAME_MAGIC {
        return abi::errno(abi::EINVAL);
    }

    let restored = KERNEL.with_task_manager(|tm| {
        let Some(t) = tm.get_current() else {
            return false;
        };
        crate::schedule::signal::restore(&mut t.write(), frame_ptr, signal_frame.registers).is_ok()
    });
    if !restored {
        return abi::errno(abi::EINVAL);
    }

    drop(process);
    task_next();
}

pub fn syscall_sigprocmask(how: u32, set: u32, old: u32) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let Some(t) = tm.get_current() else {
            return abi::errno(abi::ESRCH);
        };
        let mut t = t.write();
        let mut mask = 0u32;
        if set != 0
            && user::copy_from_user(
                &t.process.page_directory,
                set,
                &mut mask as *mut u32 as *mut u8,
                4,
            )
            .is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        if old != 0
            && user::write_value(&t.process.page_directory, old, &t.signals.blocked).is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        if set != 0 {
            mask &= !crate::schedule::signal::UNBLOCKABLE;
            t.signals.blocked = match how {
                0 => t.signals.blocked | mask,
                1 => t.signals.blocked & !mask,
                2 => mask,
                _ => return abi::errno(abi::EINVAL),
            };
        }
        0
    })
}
pub fn syscall_sigpending(ptr: u32) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let Some(t) = tm.get_current() else {
            return abi::errno(abi::ESRCH);
        };
        let t = t.read();
        let pending = (*t.process.pending_signals.lock() | t.signals.pending) & t.signals.blocked;
        if user::write_value(&t.process.page_directory, ptr, &pending).is_err() {
            abi::errno(abi::EFAULT)
        } else {
            0
        }
    })
}
pub fn syscall_sigaltstack(new: u32, old: u32) -> u32 {
    use crate::schedule::signal::AltStack;
    KERNEL.with_task_manager(|tm| {
        let Some(t) = tm.get_current() else {
            return abi::errno(abi::ESRCH);
        };
        let mut t = t.write();
        let mut stack = t.signals.alt;
        if t.signals.on_alt {
            stack.flags = 1;
        }
        let mut requested = stack;
        if new != 0
            && user::copy_from_user(
                &t.process.page_directory,
                new,
                &mut requested as *mut AltStack as *mut u8,
                12,
            )
            .is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        if old != 0 && user::write_value(&t.process.page_directory, old, &stack).is_err() {
            return abi::errno(abi::EFAULT);
        }
        if new != 0 {
            if t.signals.on_alt {
                return abi::errno(abi::EPERM);
            }
            stack = requested;
            if stack.flags & !2 != 0 {
                return abi::errno(abi::EINVAL);
            }
            if stack.flags == 0 {
                if stack.size < 2048 {
                    return abi::errno(abi::ENOMEM);
                }
                if user::validate_range(&t.process, stack.address, stack.size, true).is_err() {
                    return abi::errno(abi::EFAULT);
                }
            }
            t.signals.alt = stack;
        }
        0
    })
}
pub fn syscall_pause() -> u32 {
    let blocked = KERNEL.with_task_manager(|tm| {
        tm.block_current(crate::schedule::task::WaitReason::Signal)
            .is_ok()
    });
    if !blocked {
        return abi::errno(abi::ESRCH);
    }
    task_current_set_return_value(abi::errno(4));
    task_next()
}
