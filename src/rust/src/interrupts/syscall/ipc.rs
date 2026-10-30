//! ABI adapters for descriptor creation, vectored writes and readiness.
use super::{abi, file, user};
use crate::{
    constant::TIMER_HZ,
    fs::{Pipe, PipeEnd, descriptor::ProcessDescriptor},
    kernel::KERNEL,
    schedule::{
        process::Process,
        task::{TaskState, task_next},
    },
};
use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;
fn process() -> Option<Arc<Process>> {
    KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
}
fn install_pair(
    process: &Process,
    ptr: u32,
    flags: u32,
    endpoints: [ProcessDescriptor; 2],
    modes: [u32; 2],
) -> u32 {
    if user::validate_range(process, ptr, 8, true).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let mut fds = [-1; 2];
    let mut error = 0;
    for (i, endpoint) in endpoints.into_iter().enumerate() {
        if error != 0 {
            endpoint.close();
            continue;
        }
        match process.insert_fd_with_status_flags(endpoint.clone(), flags | modes[i]) {
            Ok(fd) => {
                fds[i] = fd;
                if flags & 0x80000 != 0 {
                    let _ = process.set_fd_flags(fd, 1);
                }
            }
            Err(_) => {
                endpoint.close();
                error = abi::errno(abi::EMFILE);
            }
        }
    }
    if error == 0 && user::write_value(&process.page_directory, ptr, &fds).is_err() {
        error = abi::errno(abi::EFAULT);
    }
    if error != 0 {
        for fd in fds {
            if let Some(d) = process.remove_fd(fd) {
                d.close();
            }
        }
    }
    error
}
pub fn syscall_pipe2(ptr: u32, flags: u32) -> u32 {
    if flags & !(0x800 | 0x80000) != 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(process) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let pipe = Arc::new(Mutex::new(Pipe::new()));
    install_pair(
        &process,
        ptr,
        flags,
        [
            ProcessDescriptor::Pipe {
                pipe: pipe.clone(),
                end: PipeEnd::Read,
            },
            ProcessDescriptor::Pipe {
                pipe,
                end: PipeEnd::Write,
            },
        ],
        [0, 1],
    )
}
pub(super) fn socketpair(
    process: &Process,
    domain: u32,
    kind: u32,
    protocol: u32,
    ptr: u32,
) -> u32 {
    let base = kind & !(0x800 | 0x80000);
    if domain != 1 || !matches!(base, 1 | 5) || protocol != 0 {
        return abi::errno(abi::ENOTSUP);
    }
    let a = Arc::new(Mutex::new(if base == 5 { Pipe::packets() } else { Pipe::new() }));
    let b = Arc::new(Mutex::new(if base == 5 { Pipe::packets() } else { Pipe::new() }));
    install_pair(
        process,
        ptr,
        kind & (0x800 | 0x80000),
        [
            ProcessDescriptor::LocalSocket {
                rx: a.clone(),
                tx: b.clone(),
            },
            ProcessDescriptor::LocalSocket { rx: b, tx: a },
        ],
        [2, 2],
    )
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct IoVec {
    base: u32,
    len: u32,
}
pub fn syscall_writev(fd: u32, ptr: u32, count: u32) -> u32 {
    if count > 1024 {
        return abi::errno(abi::EINVAL);
    }
    let Some(process) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let mut vectors = vec![IoVec::default(); count as usize];
    if user::copy_from_user(
        &process.page_directory,
        ptr,
        vectors.as_mut_ptr() as *mut u8,
        count * 8,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }
    // Bound each kernel allocation. A short write is legal; callers retry remaining iovecs.
    let mut data = Vec::new();
    for v in vectors {
        let len = (v.len as usize).min(65536 - data.len());
        if len == 0 {
            continue;
        }
        let start = data.len();
        data.resize(start + len, 0);
        if user::copy_from_user(
            &process.page_directory,
            v.base,
            data[start..].as_mut_ptr(),
            len as u32,
        )
        .is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        if data.len() == 65536 {
            break;
        }
    }
    file::finish_write(file::write_buffer(process, fd as i32, data))
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PollFd {
    fd: i32,
    events: u16,
    revents: u16,
}
struct PollDeadline;
impl Drop for PollDeadline {
    fn drop(&mut self) {
        KERNEL.with_task_manager(|tm| {
            if let Some(task) = tm.get_current() { task.write().poll_deadline = None; }
        });
    }
}
pub fn syscall_poll(ptr: u32, count: u32, timeout: u32) -> u32 {
    // Every return, including an error on a restarted call, ends this deadline.
    let deadline = PollDeadline;
    if count > 128 {
        return abi::errno(abi::EINVAL);
    }
    let Some(process) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let mut fds = vec![PollFd::default(); count as usize];
    if user::validate_range(&process, ptr, count * 8, true).is_err()
        || user::copy_from_user(
            &process.page_directory,
            ptr,
            fds.as_mut_ptr() as *mut u8,
            count * 8,
        )
        .is_err()
    {
        return abi::errno(abi::EFAULT);
    }
    let mut ready = 0;
    for fd in &mut fds {
        fd.revents = if fd.fd < 0 {
            0
        } else {
            process
                .get_fd(fd.fd)
                .map(|d| d.poll(fd.events))
                .unwrap_or(32)
        };
        if fd.revents != 0 {
            ready += 1;
        }
    }
    let complete = KERNEL.with_task_manager(|tm| {
        let now = tm.get_tick();
        let current = tm.get_current().unwrap();
        let mut task = current.write();
        let deadline = *task.poll_deadline.get_or_insert_with(|| {
            if (timeout as i32) < 0 {
                u64::MAX
            } else {
                now.saturating_add((timeout as u64 * TIMER_HZ as u64 + 999) / 1000)
            }
        });
        if ready != 0 || now >= deadline {
            task.poll_deadline = None;
            true
        } else {
            task.registers.ip -= 2;
            task.restarting=true;
            task.state = TaskState::Sleeping {
                wake_tick: now.saturating_add(1),
            };
            false
        }
    });
    if complete {
        if user::copy_to_user(
            &process.page_directory,
            ptr,
            fds.as_ptr() as *const u8,
            count * 8,
        )
        .is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        ready
    } else {
        drop(fds);
        drop(process);
        // This syscall is suspended, rather than completed. Retain its deadline.
        core::mem::forget(deadline);
        task_next();
    }
}
