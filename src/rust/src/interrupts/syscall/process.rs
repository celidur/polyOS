use alloc::{string::String, string::ToString, vec::Vec};

use crate::{
    constant::MAX_PATH,
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::{
        process::ProcessArguments,
        process_manager::process_terminate,
        task::{WaitReason, task_next},
    },
};

use super::{abi, user};

const WNOHANG: u32 = 1;
const MAX_EXEC_STRINGS: u32 = 512;
const MAX_EXEC_STRING_LEN: usize = 1024;

pub fn syscall_waitpid(_frame: &InterruptFrame) -> u32 {
    let args = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        let child_pid = task.get_stack_item(0);
        let status_ptr = task.get_stack_item(1);
        let options = task.get_stack_item(2);
        Some((child_pid, status_ptr, options))
    });

    let Some((child_pid, status_ptr, options)) = args else {
        return abi::errno(abi::ECHILD);
    };

    if child_pid == 0 || options & !WNOHANG != 0 {
        return abi::errno(abi::EINVAL);
    }

    wait_for_process_unix(child_pid, status_ptr, options & WNOHANG != 0)
}

pub fn syscall_execve(_frame: &InterruptFrame) -> u32 {
    let exec = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();

        let path_ptr = task.get_stack_item(0);
        let argv_ptr = task.get_stack_item(1);
        let envp_ptr = task.get_stack_item(2);
        if path_ptr == 0 {
            return None;
        }

        let raw_path = user::read_c_string(&task, path_ptr, MAX_PATH)?;
        let path = task.process.resolve_path(raw_path.as_str())?;
        let args = read_argv_from_task(&task, raw_path.as_str(), argv_ptr)?;
        let env = if envp_ptr == 0 {
            task.process.env.lock().clone()
        } else {
            read_string_array_from_task(&task, envp_ptr)?
        };
        Some((task.process.pid, path, ProcessArguments { args, env }))
    });

    let Some((pid, path, args)) = exec else {
        return abi::errno(abi::EFAULT);
    };

    match KERNEL.with_process_manager(|pm| pm.exec(pid, path.as_str(), args)) {
        Ok(()) => {
            drop(path);
            task_next();
        }
        Err(error) => {
            serial_println!("execve({}) failed: {:?}", path, error);
            abi::errno(abi::ENOENT)
        }
    }
}

pub fn syscall_fork(_frame: &InterruptFrame) -> u32 {
    let fork_context = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        let mut child_registers = task.registers;
        child_registers.eax = 0;
        Some((task.process.clone(), child_registers, task.priority))
    });

    let Some((parent, child_registers, priority)) = fork_context else {
        return abi::errno(abi::EAGAIN);
    };

    match KERNEL.with_process_manager(|pm| pm.fork(parent, child_registers, priority)) {
        Ok(pid) => pid,
        Err(error) => {
            serial_println!("fork failed: {:?}", error);
            abi::errno(abi::EAGAIN)
        }
    }
}

pub fn syscall_exit(_frame: &InterruptFrame) -> u32 {
    let code = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(0) as i32)
    });

    process_terminate(code.unwrap_or(0));

    task_next();
}

pub fn syscall_getpid(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|task| task.read().process.pid)
            .unwrap_or_else(|| abi::errno(abi::ESRCH))
    })
}

pub fn syscall_getppid(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|task| task.read().process.parent_pid().unwrap_or(0))
            .unwrap_or_else(|| abi::errno(abi::ESRCH))
    })
}

fn wait_for_process_unix(child_pid: u32, status_ptr: u32, no_hang: bool) -> u32 {
    wait_for_process(child_pid, status_ptr, no_hang)
}

fn wait_for_process(child_pid: u32, status_ptr: u32, no_hang: bool) -> u32 {
    let wait_context = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.id, task.process.pid))
    });

    let Some((task_id, parent_pid)) = wait_context else {
        return abi::errno(abi::ECHILD);
    };

    let wait = KERNEL.with_process_manager(|pm| {
        let Some(parent) = pm.get(parent_pid) else {
            return Err(crate::error::KernelError::NoTasks);
        };

        let is_child = parent.children.lock().contains(&child_pid);
        if !is_child {
            return Err(crate::error::KernelError::NoTasks);
        }

        let waiter = (!no_hang).then_some((task_id, status_ptr, child_pid));
        pm.wait_for_exit(parent_pid, child_pid, waiter)
    });

    match wait {
        Ok(Some(status)) => {
            if status_ptr != 0 && write_status_to_current(status_ptr, status).is_err() {
                return abi::errno(abi::EFAULT);
            }

            child_pid
        }
        Ok(None) if no_hang => 0,
        Ok(None) => {
            let blocked = KERNEL.with_task_manager(|tm| {
                tm.block_current(WaitReason::Process(child_pid as usize))
                    .is_ok()
            });

            if !blocked {
                return abi::errno(abi::ECHILD);
            }

            task_next();
        }
        Err(_) => abi::errno(abi::ECHILD),
    }
}

fn write_status_to_current(status_ptr: u32, status: i32) -> Result<(), ()> {
    KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current().ok_or(())?;
        let task = current_task.read();
        user::write_value(&task.process.page_directory, status_ptr, &status)
    })
}

fn read_argv_from_task(
    task: &crate::schedule::task::Task,
    path: &str,
    argv_ptr: u32,
) -> Option<Vec<String>> {
    if argv_ptr == 0 {
        return Some(vec![path.to_string()]);
    }

    let mut args = read_string_array_from_task(task, argv_ptr)?;

    if args.is_empty() {
        args.push(path.to_string());
    }

    Some(args)
}

fn read_string_array_from_task(
    task: &crate::schedule::task::Task,
    array_ptr: u32,
) -> Option<Vec<String>> {
    let mut values = Vec::new();
    for index in 0..MAX_EXEC_STRINGS {
        let item_ptr = user::read_u32(task, array_ptr.checked_add(index * 4_u32)?)?;
        if item_ptr == 0 {
            return Some(values);
        }

        values.push(user::read_c_string(task, item_ptr, MAX_EXEC_STRING_LEN)?);
    }

    None
}
