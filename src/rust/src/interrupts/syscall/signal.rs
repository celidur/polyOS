use crate::{
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::{
        process::{SIGKILL, SIGNAL_FRAME_MAGIC, SIGSTOP, SignalAction, SignalFrame, valid_signal},
        process_manager::SignalEffect,
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

pub fn syscall_kill(_frame: &InterruptFrame) -> u32 {
    let args = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((
            task.process.pid,
            task.get_stack_item(0) as i32,
            task.get_stack_item(1) as i32,
        ))
    });

    let Some((current_pid, pid, signal)) = args else {
        return abi::errno(abi::ESRCH);
    };

    if pid <= 0 {
        return abi::errno(abi::ENOTSUP);
    }
    if signal < 0 || (signal != 0 && !valid_signal(signal as u32)) {
        return abi::errno(abi::EINVAL);
    }

    let target_pid = pid as u32;
    if target_pid == current_pid {
        task_current_set_return_value(0);
    }

    let result = KERNEL.with_process_manager(|pm| pm.signal(target_pid, signal as u32));
    match result {
        Ok(effect) => {
            if target_pid == current_pid
                && matches!(effect, SignalEffect::Delivered | SignalEffect::Terminated)
            {
                task_next();
            }
            0
        }
        Err(_) => abi::errno(abi::ESRCH),
    }
}

pub fn syscall_sigaction(_frame: &InterruptFrame) -> u32 {
    let args = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((
            task.process.clone(),
            task.get_stack_item(0),
            task.get_stack_item(1),
            task.get_stack_item(2),
        ))
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

    if oldact_ptr != 0 {
        let user_old: UserSignalAction = old_action.into();
        if user::write_value(&process.page_directory, oldact_ptr, &user_old).is_err() {
            return abi::errno(abi::EFAULT);
        }
    }

    if act_ptr != 0 {
        let mut user_action = UserSignalAction::default();
        if user::copy_from_user(
            &process.page_directory,
            act_ptr,
            &mut user_action as *mut UserSignalAction as *mut u8,
            core::mem::size_of::<UserSignalAction>() as u32,
        )
        .is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        if user_action.handler > 1 && user_action.restorer == 0 {
            return abi::errno(abi::EINVAL);
        }

        let _ = process.set_signal_action(signal, user_action.into());
    }

    0
}

pub fn syscall_sigreturn(_frame: &InterruptFrame) -> u32 {
    let Some((process, frame_ptr)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), task.get_stack_item(0)))
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

    KERNEL.with_task_manager(|tm| {
        if let Some(current_task) = tm.get_current() {
            current_task.write().registers = signal_frame.registers;
        }
    });

    drop(process);
    task_next();
}
