use alloc::{string::ToString, vec::Vec};

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
        return abi::error();
    };

    if child_pid == 0 || options != 0 {
        return abi::error();
    }

    wait_for_process_unix(child_pid, status_ptr)
}

pub fn syscall_execve(_frame: &InterruptFrame) -> u32 {
    let exec = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();

        let path_ptr = task.get_stack_item(0);
        let argv_ptr = task.get_stack_item(1);
        let _envp_ptr = task.get_stack_item(2);
        if path_ptr == 0 {
            return None;
        }

        let path = user::read_c_string(&task, path_ptr, MAX_PATH)?;
        let args = read_argv_from_task(&task, path.as_str(), argv_ptr)?;
        Some((task.process.pid, path, ProcessArguments { args }))
    });

    let Some((pid, path, args)) = exec else {
        return abi::error();
    };

    match KERNEL.with_process_manager(|pm| pm.exec(pid, path.as_str(), args)) {
        Ok(()) => task_next(),
        Err(error) => {
            serial_println!("execve({}) failed: {:?}", path, error);
            abi::error()
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
        return abi::error();
    };

    match KERNEL.with_process_manager(|pm| pm.fork(parent, child_registers, priority)) {
        Ok(pid) => pid,
        Err(error) => {
            serial_println!("fork failed: {:?}", error);
            abi::error()
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
            .unwrap_or_else(abi::error)
    })
}

pub fn syscall_getppid(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|task| task.read().process.parent.unwrap_or(0))
            .unwrap_or_else(abi::error)
    })
}

fn wait_for_process_unix(child_pid: u32, status_ptr: u32) -> u32 {
    wait_for_process(child_pid, status_ptr, child_pid)
}

fn wait_for_process(child_pid: u32, status_ptr: u32, blocked_return_value: u32) -> u32 {
    let wait_context = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.id, task.process.pid))
    });

    let Some((task_id, parent_pid)) = wait_context else {
        return abi::error();
    };

    let wait = KERNEL.with_process_manager(|pm| {
        let Some(parent) = pm.get(parent_pid) else {
            return Err(crate::error::KernelError::NoTasks);
        };

        let is_child = parent.children.lock().contains(&child_pid);
        if !is_child {
            return Err(crate::error::KernelError::NoTasks);
        }

        pm.wait_for_exit(
            parent_pid,
            child_pid,
            task_id,
            status_ptr,
            blocked_return_value,
        )
    });

    match wait {
        Ok(Some(status)) => {
            if status_ptr != 0 && write_status_to_current(status_ptr, status).is_err() {
                return abi::error();
            }

            if blocked_return_value == 0 {
                status as u32
            } else {
                blocked_return_value
            }
        }
        Ok(None) => {
            let blocked = KERNEL.with_task_manager(|tm| {
                tm.block_current(WaitReason::Process(child_pid as usize))
                    .is_ok()
            });

            if !blocked {
                return abi::error();
            }

            task_next();
        }
        Err(_) => abi::error(),
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
) -> Option<Vec<alloc::string::String>> {
    if argv_ptr == 0 {
        return Some(vec![path.to_string()]);
    }

    let mut args = Vec::new();
    for index in 0..128 {
        let arg_ptr = user::read_u32(task, argv_ptr.checked_add(index * 4_u32)?)?;
        if arg_ptr == 0 {
            break;
        }

        args.push(user::read_c_string(task, arg_ptr, 512)?);
    }

    if args.is_empty() {
        args.push(path.to_string());
    }

    Some(args)
}
