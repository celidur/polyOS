use alloc::{string::String, string::ToString, vec::Vec};

use crate::{
    constant::MAX_PATH,
    fs::FsError,
    kernel::KERNEL,
    schedule::{
        process::{ACCESS_EXECUTE, ProcessArguments},
        task::{WaitReason, task_next},
    },
};

use super::{abi, user};

const MAX_EXEC_STRINGS: u32 = 512;
const MAX_EXEC_STRING_LEN: usize = 1024;

pub fn syscall_waitpid(selector: u32, status: u32, options: u32) -> u32 {
    if options & !(1 | 2 | 8) != 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    if status != 0 && user::validate_range(&p, status, 4, true).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let result = KERNEL.with_process_manager(|pm| pm.wait_event(p.pid, selector as i32, options));
    match result {
        Ok(Some((pid, value, _uid))) => {
            if status != 0 && user::write_value(&p.page_directory, status, &value).is_err() {
                abi::errno(abi::EFAULT)
            } else {
                pid
            }
        }
        Ok(None) if options & 1 != 0 => 0,
        Ok(None) => {
            KERNEL.with_task_manager(|tm| {
                let _ = tm.block_current_and_restart_syscall(WaitReason::Child(p.pid));
            });
            drop(p);
            task_next()
        }
        Err(e) => abi::errno(e),
    }
}

pub fn syscall_execve(path_ptr: u32, argv_ptr: u32, envp_ptr: u32) -> u32 {
    let exec = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();

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
        Some((
            task.process.clone(),
            task.process.pid,
            path,
            ProcessArguments { args, env },
        ))
    });

    let Some((process, pid, path, args)) = exec else {
        return abi::errno(abi::EFAULT);
    };

    let path = match process.follow_path(&path) {
        Ok(p) => p,
        Err(e) => return super::file::fs_errno(e),
    };
    match KERNEL.vfs.read().stat(path.as_str()) {
        Ok(metadata) if metadata.is_dir => return abi::errno(abi::EACCES),
        Ok(metadata) if !process.has_permission(&metadata, ACCESS_EXECUTE) => {
            return abi::errno(abi::EACCES);
        }
        Ok(_) => {}
        Err(FsError::NotFound) => return abi::errno(abi::ENOENT),
        Err(_) => return abi::errno(abi::EACCES),
    }

    match KERNEL.with_process_manager(|pm| pm.exec(pid, path.as_str(), args)) {
        Ok(()) => {
            drop(process);
            drop(path);
            task_next();
        }
        Err(error) => {
            serial_println!("execve({}) failed: {:?}", path, error);
            abi::errno(match error {
                crate::error::KernelError::InvalidExecutable => abi::ENOEXEC,
                crate::error::KernelError::Allocation => abi::ENOMEM,
                _ => abi::EIO,
            })
        }
    }
}

pub fn syscall_fork() -> u32 {
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

pub fn syscall_getpid() -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|task| task.read().process.pid)
            .unwrap_or_else(|| abi::errno(abi::ESRCH))
    })
}

pub fn syscall_getuid() -> u32 {
    current_process_id_field(|process| process.uid())
}

pub fn syscall_getppid() -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|task| task.read().process.parent_pid().unwrap_or(0))
            .unwrap_or_else(|| abi::errno(abi::ESRCH))
    })
}

pub fn syscall_getgid() -> u32 {
    current_process_id_field(|process| process.gid())
}

pub fn syscall_geteuid() -> u32 {
    current_process_id_field(|process| process.euid())
}

pub fn syscall_getegid() -> u32 {
    current_process_id_field(|process| process.egid())
}

fn current_process_id_field(read: impl FnOnce(&crate::schedule::process::Process) -> u32) -> u32 {
    KERNEL.with_task_manager(|tm| {
        tm.get_current()
            .map(|task| read(&task.read().process))
            .unwrap_or_else(|| abi::errno(abi::ESRCH))
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
