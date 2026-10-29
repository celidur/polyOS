use super::{abi, user};
use crate::{constant::MAX_PATH, kernel::KERNEL, schedule::process::Process};
use alloc::sync::Arc;
fn current() -> Option<Arc<Process>> {
    KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
}
fn result(r: Result<u32, i32>) -> u32 {
    r.unwrap_or_else(abi::errno)
}
pub fn syscall_setuid(id: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    result(p.control.lock().credentials.set_uid(id).map(|_| 0))
}
pub fn syscall_setgid(id: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    result(p.control.lock().credentials.set_gid(id).map(|_| 0))
}
pub fn syscall_setgroups(count: u32, ptr: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    if p.euid() != 0 {
        return abi::errno(abi::EPERM);
    }
    if count > 32 {
        return abi::errno(abi::EINVAL);
    }
    let mut groups = vec![0u32; count as usize];
    if user::copy_from_user(
        &p.page_directory,
        ptr,
        groups.as_mut_ptr() as *mut u8,
        count * 4,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }
    p.control.lock().credentials.groups = groups;
    0
}
pub fn syscall_getgroups(count: u32, ptr: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    let groups = p.control.lock().credentials.groups.clone();
    if count == 0 {
        return groups.len() as u32;
    }
    if count < groups.len() as u32 {
        return abi::errno(abi::EINVAL);
    }
    if user::copy_to_user(
        &p.page_directory,
        ptr,
        groups.as_ptr() as *const u8,
        groups.len() as u32 * 4,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }
    groups.len() as u32
}
pub fn syscall_chroot(ptr: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    if p.euid() != 0 {
        return abi::errno(abi::EPERM);
    }
    let Some(name) = user::read_process_string(&p, ptr, MAX_PATH) else {
        return abi::errno(abi::EFAULT);
    };
    let Some(path) = p.resolve_path(&name) else {
        return abi::errno(abi::ENOENT);
    };
    match KERNEL.vfs.read().stat(&path) {
        Ok(m) if !m.is_dir => return abi::errno(abi::ENOTDIR),
        Ok(m) if !p.has_permission(&m, 1) => return abi::errno(abi::EACCES),
        Err(e) => return super::file::fs_errno(e),
        _ => {}
    }
    p.control.lock().root = path.clone();
    p.set_cwd(path);
    0
}
pub fn syscall_setpgid(pid: u32, group: u32) -> u32 {
    if (pid as i32) < 0 || (group as i32) < 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    let pid = if pid == 0 { p.pid } else { pid };
    let group = if group == 0 { pid } else { group };
    result(
        KERNEL
            .with_process_manager(|pm| pm.setpgid(p.pid, pid, group))
            .map(|_| 0),
    )
}
pub fn syscall_getpgid(pid: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    let target = if pid == 0 {
        Some(p)
    } else {
        KERNEL.with_process_manager(|pm| pm.get(pid))
    };
    target
        .map(|p| p.control.lock().pgid)
        .unwrap_or_else(|| abi::errno(abi::ESRCH))
}
pub fn syscall_getpgrp() -> u32 {
    syscall_getpgid(0)
}
pub fn syscall_setsid() -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    result(KERNEL.with_process_manager(|pm| pm.setsid(p.pid)))
}
pub fn syscall_getsid(pid: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    let target = if pid == 0 {
        Some(p)
    } else {
        KERNEL.with_process_manager(|pm| pm.get(pid))
    };
    target
        .map(|p| p.control.lock().sid)
        .unwrap_or_else(|| abi::errno(abi::ESRCH))
}
pub fn syscall_prctl(option: u32, arg: u32, third: u32, fourth: u32, fifth: u32) -> u32 {
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    match option {
        15 => {
            let text = user::read_process_string(&p, arg, 16);
            let mut name = [0u8; 16];
            if let Some(text) = text {
                let n = text.len().min(15);
                name[..n].copy_from_slice(&text.as_bytes()[..n]);
            } else if user::copy_from_user(&p.page_directory, arg, name.as_mut_ptr(), 15).is_err() {
                return abi::errno(abi::EFAULT);
            }
            KERNEL.with_task_manager(|tm| {
                if let Some(t) = tm.get_current() {
                    t.write().name = name;
                }
            });
            0
        }
        16 => {
            let name = KERNEL.with_task_manager(|tm| tm.get_current().unwrap().read().name);
            if user::write_value(&p.page_directory, arg, &name).is_err() {
                abi::errno(abi::EFAULT)
            } else {
                0
            }
        }
        38 => {
            if arg != 1 || third | fourth | fifth != 0 {
                return abi::errno(abi::EINVAL);
            }
            p.control.lock().no_new_privs = true;
            0
        }
        39 => {
            if arg | third | fourth | fifth != 0 {
                return abi::errno(abi::EINVAL);
            }
            p.control.lock().no_new_privs as u32
        }
        _ => abi::errno(abi::ENOTSUP),
    }
}

pub fn syscall_waitid(which: u32, id: u32, info: u32, options: u32, usage: u32) -> u32 {
    if options & !(1 | 2 | 4 | 8) != 0 || options & 4 == 0 {
        return abi::errno(abi::EINVAL);
    }
    if usage != 0 {
        return abi::errno(abi::ENOTSUP);
    }
    let selector = match which {
        0 => -1,
        1 if id > 0 && id <= i32::MAX as u32 => id as i32,
        2 if id <= i32::MAX as u32 => -(id as i32),
        _ => return abi::errno(abi::EINVAL),
    };
    let Some(p) = current() else {
        return abi::errno(abi::ESRCH);
    };
    if user::validate_range(&p, info, 128, true).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let outcome = KERNEL.with_process_manager(|pm| pm.wait_event(p.pid, selector, options & !4));
    let mut data = [0u32; 32];
    match outcome {
        Ok(Some((pid, status, uid))) => {
            data[0] = 17;
            data[3] = pid;
            data[4] = uid;
            let (code, value) = if status == 0xffff {
                (6, 18)
            } else if status & 255 == 0x7f {
                (5, (status >> 8) & 255)
            } else if status & 127 != 0 {
                (2, status & 127)
            } else {
                (1, (status >> 8) & 255)
            };
            data[2] = code;
            data[5] = value as u32;
        }
        Ok(None) if options & 1 != 0 => {}
        Ok(None) => {
            KERNEL.with_task_manager(|tm| {
                let _ = tm.block_current_and_restart_syscall(
                    crate::schedule::task::WaitReason::Child(p.pid),
                );
            });
            drop(p);
            crate::schedule::task::task_next()
        }
        Err(e) => return abi::errno(e),
    }
    if user::write_value(&p.page_directory, info, &data).is_err() {
        abi::errno(abi::EFAULT)
    } else {
        0
    }
}
