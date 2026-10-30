use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Mutex;

use crate::{
    constant::MAX_PATH,
    fs::{FsError, Pipe, PipeEnd, PipeError, file::FileStat},
    kernel::KERNEL,
    schedule::{
        process::{ACCESS_EXECUTE, FD_CLOEXEC, O_NONBLOCK, Process, ProcessDescriptor},
        task::{Task, TaskId, WaitReason, task_next},
    },
};

use super::{abi, user};

const O_APPEND: u32 = 0x400;
const SEEK_SET: u32 = 0;
const SEEK_END: u32 = 2;
const F_DUPFD: u32 = 0;
const F_DUPFD_CLOEXEC: u32 = 1030;
const F_GETFD: u32 = 1;
const F_SETFD: u32 = 2;
const F_GETFL: u32 = 3;
const F_SETFL: u32 = 4;

const DT_UNKNOWN: u8 = 0;
const DT_REG: u8 = 8;
const DT_DIR: u8 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct Dirent {
    d_ino: u32,
    d_off: u32,
    d_reclen: u16,
    d_type: u8,
    d_name: [u8; 256],
}

enum PipeSyscallResult {
    Completed(u32),
    Block(WaitReason),
}

pub fn syscall_open(path_ptr: u32, flags: u32, mode: u32) -> u32 {
    super::filesystem::syscall_openat((-100i32) as u32, path_ptr, flags, mode)
}

pub fn syscall_read(fd: u32, buf_ptr: u32, len: u32) -> u32 {
    let Some((process, fd, buf_ptr, len)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            fd as i32,
            buf_ptr,
            len.min(65536) as usize,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    if user::validate_range(&process, buf_ptr, len as u32, true).is_err() {
        return abi::errno(abi::EFAULT);
    }

    if process
        .get_status_flags(fd)
        .map(|f| f & 3 == 1)
        .unwrap_or(false)
    {
        return abi::errno(abi::EBADF);
    }
    let descriptor = match process.get_fd(fd) {
        Some(descriptor) => descriptor,
        None => return abi::errno(abi::EBADF),
    };

    if let Some((pipe, end)) = descriptor.pipe_endpoint(false) {
        let result = syscall_pipe_read(&process, fd, pipe, end, buf_ptr, len);
        return match result {
            PipeSyscallResult::Completed(value) => value,
            PipeSyscallResult::Block(reason) => {
                drop(descriptor);
                drop(process);
                block_current_and_restart(reason)
            }
        };
    }

    let mut data = vec![0; len];
    let operation = if process.get_status_flags(fd).unwrap_or(0) & O_NONBLOCK != 0 {
        descriptor.read_nonblocking(data.as_mut_slice())
    } else {
        descriptor.read(data.as_mut_slice())
    };
    let read = match operation {
        Ok(read) => read,
        Err(FsError::WouldBlock) if process.get_status_flags(fd).unwrap_or(0) & O_NONBLOCK == 0 => {
            let pending = KERNEL.with_task_manager(|tm| {
                tm.get_current().is_some_and(|t| {
                    let t = t.read();
                    (t.signals.pending | *t.process.pending_signals.lock()) & !t.signals.blocked
                        != 0
                })
            });
            if pending {
                return abi::errno(4);
            }
            drop(descriptor);
            drop(data);
            drop(process);
            KERNEL.with_task_manager(|tm| {
                let now = tm.get_tick();
                if let Some(t) = tm.get_current() {
                    let mut t = t.write();
                    t.registers.ip -= 2;
                    t.restarting = true;
                    t.state = crate::schedule::task::TaskState::Sleeping { wake_tick: now + 1 };
                }
            });
            task_next()
        }
        Err(error) => return fs_errno(error),
    };

    if read != 0
        && user::copy_to_user(&process.page_directory, buf_ptr, data.as_ptr(), read as u32).is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    read as u32
}

pub fn syscall_write(fd: u32, buf_ptr: u32, len: u32) -> u32 {
    let Some((process, fd, ptr, len)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            fd as i32,
            buf_ptr,
            len.min(65536) as usize,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    if ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let mut data = vec![0; len];
    if len != 0
        && user::copy_from_user(&process.page_directory, ptr, data.as_mut_ptr(), len as u32)
            .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    finish_write(write_buffer(process, fd, data))
}

pub(super) fn write_buffer(process: Arc<Process>, fd: i32, data: Vec<u8>) -> u32 {
    if process
        .get_status_flags(fd)
        .map(|f| f & 3 == 0)
        .unwrap_or(false)
    {
        return abi::errno(abi::EBADF);
    }
    let Some(descriptor) = process.get_fd(fd) else {
        return abi::errno(abi::EBADF);
    };

    if let Some((pipe, end)) = descriptor.pipe_endpoint(true) {
        let result = syscall_pipe_write(&process, fd, pipe, end, data.as_slice());
        return match result {
            PipeSyscallResult::Completed(value) => value,
            PipeSyscallResult::Block(reason) => {
                drop(descriptor);
                drop(data);
                drop(process);
                block_current_and_restart(reason)
            }
        };
    }

    if process.get_status_flags(fd).unwrap_or(0) & O_APPEND != 0 {
        if let Err(error) = seek_for_append(&descriptor) {
            return fs_errno(error);
        }
    }

    match descriptor.write(data.as_slice()) {
        Ok(written) => written as u32,
        Err(error) => fs_errno(error),
    }
}

fn syscall_pipe_read(
    process: &Process,
    fd: i32,
    pipe: Arc<Mutex<Pipe>>,
    end: PipeEnd,
    buf_ptr: u32,
    len: usize,
) -> PipeSyscallResult {
    let task_id = current_task_id();
    let mut data = vec![0; len];

    let (result, waiters, pipe_id, should_block) = {
        let mut pipe = pipe.lock();
        let pipe_id = pipe.id();
        match pipe.read(end, data.as_mut_slice()) {
            Ok(read) => {
                let waiters = if read > 0 || pipe.is_packet() {
                    pipe.take_write_waiters()
                } else {
                    Vec::new()
                };
                (Ok(read), waiters, pipe_id, false)
            }
            Err(PipeError::WouldBlock)
                if process.get_status_flags(fd).unwrap_or(0) & O_NONBLOCK == 0 =>
            {
                if let Some(task_id) = task_id {
                    pipe.add_read_waiter(task_id);
                    (Err(PipeError::WouldBlock), Vec::new(), pipe_id, true)
                } else {
                    (Err(PipeError::WouldBlock), Vec::new(), pipe_id, false)
                }
            }
            Err(error) => (Err(error), Vec::new(), pipe_id, false),
        }
    };

    wake_pipe_waiters(waiters);

    match result {
        Ok(read) => {
            if read != 0
                && user::copy_to_user(&process.page_directory, buf_ptr, data.as_ptr(), read as u32)
                    .is_err()
            {
                return PipeSyscallResult::Completed(abi::errno(abi::EFAULT));
            }

            PipeSyscallResult::Completed(read as u32)
        }
        Err(PipeError::WouldBlock) if should_block => {
            PipeSyscallResult::Block(WaitReason::PipeRead(pipe_id))
        }
        Err(error) => PipeSyscallResult::Completed(fs_errno(pipe_fs_error(error))),
    }
}

fn syscall_pipe_write(
    process: &Process,
    fd: i32,
    pipe: Arc<Mutex<Pipe>>,
    end: PipeEnd,
    data: &[u8],
) -> PipeSyscallResult {
    let task_id = current_task_id();

    let (result, waiters, pipe_id, should_block) = {
        let mut pipe = pipe.lock();
        let pipe_id = pipe.id();
        match pipe.write(end, data) {
            Ok(written) => {
                let waiters = if written > 0 || pipe.is_packet() {
                    pipe.take_read_waiters()
                } else {
                    Vec::new()
                };
                (Ok(written), waiters, pipe_id, false)
            }
            Err(PipeError::WouldBlock)
                if process.get_status_flags(fd).unwrap_or(0) & O_NONBLOCK == 0 =>
            {
                if let Some(task_id) = task_id {
                    pipe.add_write_waiter(task_id);
                    (Err(PipeError::WouldBlock), Vec::new(), pipe_id, true)
                } else {
                    (Err(PipeError::WouldBlock), Vec::new(), pipe_id, false)
                }
            }
            Err(error) => (Err(error), Vec::new(), pipe_id, false),
        }
    };

    wake_pipe_waiters(waiters);

    match result {
        Ok(written) => PipeSyscallResult::Completed(written as u32),
        Err(PipeError::WouldBlock) if should_block => {
            PipeSyscallResult::Block(WaitReason::PipeWrite(pipe_id))
        }
        Err(error) => PipeSyscallResult::Completed(fs_errno(pipe_fs_error(error))),
    }
}

pub fn syscall_lseek(fd: u32, offset: u32, whence: u32) -> u32 {
    let Some((process, fd, offset, whence)) =
        with_current_task(|task| Some((task.process.clone(), fd as i32, offset, whence)))
    else {
        return abi::errno(abi::EFAULT);
    };

    let Some(descriptor) = process.get_fd(fd) else {
        return abi::errno(abi::EBADF);
    };

    let position = match whence {
        SEEK_SET => offset as i32 as i64,
        1 => match descriptor.position() {
            Ok(pos) => pos as i64 + offset as i32 as i64,
            Err(error) => return fs_errno(error),
        },
        SEEK_END => match descriptor.stat() {
            Ok(meta) => meta.size as i64 + offset as i32 as i64,
            Err(error) => return fs_errno(error),
        },
        _ => return abi::errno(abi::EINVAL),
    };

    if position < 0 || position > i32::MAX as i64 {
        return abi::errno(abi::EINVAL);
    }

    match descriptor.seek(position as usize) {
        Ok(pos) => pos as u32,
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_fstat(fd: u32, stat_ptr: u32) -> u32 {
    let Some((process, fd, stat_ptr)) =
        with_current_task(|task| Some((task.process.clone(), fd as i32, stat_ptr)))
    else {
        return abi::errno(abi::EFAULT);
    };

    if stat_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let Some(descriptor) = process.get_fd(fd) else {
        return abi::errno(abi::EBADF);
    };

    let meta = match descriptor.stat() {
        Ok(meta) => meta,
        Err(error) => return fs_errno(error),
    };

    let stat = metadata_to_stat(&meta);

    if user::write_value(&process.page_directory, stat_ptr, &stat).is_err() {
        return abi::errno(abi::EFAULT);
    }

    0
}

pub fn syscall_close(fd: u32) -> u32 {
    let Some((process, fd)) = with_current_task(|task| Some((task.process.clone(), fd as i32)))
    else {
        return abi::errno(abi::EFAULT);
    };

    match process.remove_fd(fd) {
        Some(descriptor) => {
            descriptor.close();
            0
        }
        None => abi::errno(abi::EBADF),
    }
}

pub fn syscall_dup(fd: u32) -> u32 {
    let Some((process, fd)) = with_current_task(|task| Some((task.process.clone(), fd as i32)))
    else {
        return abi::errno(abi::EFAULT);
    };

    match process.duplicate_fd(fd) {
        Ok(new_fd) => new_fd as u32,
        Err(e) => fs_errno(e),
    }
}

pub fn syscall_dup2(oldfd: u32, newfd: u32) -> u32 {
    let Some((process, old_fd, new_fd)) =
        with_current_task(|task| Some((task.process.clone(), oldfd as i32, newfd as i32)))
    else {
        return abi::errno(abi::EFAULT);
    };

    match process.duplicate_fd_to(old_fd, new_fd) {
        Ok(fd) => fd as u32,
        Err(_) => abi::errno(abi::EBADF),
    }
}

pub fn syscall_fcntl(fd: u32, cmd: u32, arg: u32) -> u32 {
    let Some((process, fd, cmd, arg)) =
        with_current_task(|task| Some((task.process.clone(), fd as i32, cmd, arg)))
    else {
        return abi::errno(abi::EFAULT);
    };

    match cmd {
        F_DUPFD | F_DUPFD_CLOEXEC => match process.duplicate_fd_from_flags(
            fd,
            arg as i32,
            if cmd == F_DUPFD_CLOEXEC {
                FD_CLOEXEC
            } else {
                0
            },
        ) {
            Ok(new_fd) => new_fd as u32,
            Err(e) => fs_errno(e),
        },
        F_GETFD => process
            .get_fd_flags(fd)
            .map(|flags| flags & FD_CLOEXEC)
            .unwrap_or_else(|| abi::errno(abi::EBADF)),
        F_SETFD => match process.set_fd_flags(fd, arg & FD_CLOEXEC) {
            Ok(()) => 0,
            Err(_) => abi::errno(abi::EBADF),
        },
        F_GETFL => process
            .get_status_flags(fd)
            .unwrap_or_else(|| abi::errno(abi::EBADF)),
        F_SETFL => match process.set_status_flags(fd, arg) {
            Ok(()) => 0,
            Err(_) => abi::errno(abi::EBADF),
        },
        _ => abi::errno(abi::EINVAL),
    }
}

pub fn syscall_pipe(pipefd_ptr: u32) -> u32 {
    super::ipc::syscall_pipe2(pipefd_ptr, 0)
}

pub fn syscall_unlink(path_ptr: u32) -> u32 {
    remove_path(path_ptr, false)
}

pub fn syscall_rmdir(path_ptr: u32) -> u32 {
    remove_path(path_ptr, true)
}

pub fn syscall_mkdir(path_ptr: u32, mode: u32) -> u32 {
    let (process, path) = match current_path(path_ptr) {
        Ok(context) => context,
        Err(e) => return fs_errno(e),
    };
    let mode = mode as u16;

    if let Err(e) = crate::fs::access::parent_write(&process, &path) {
        return fs_errno(e);
    }
    match KERNEL.vfs.read().create(path.as_str(), true) {
        Ok(()) => {
            apply_created_mode(&process, path.as_str(), mode, true);
            0
        }
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_chmod(path_ptr: u32, mode: u32) -> u32 {
    let (process, path) = match current_path(path_ptr) {
        Ok(context) => context,
        Err(e) => return fs_errno(e),
    };
    let path = match process.follow_path(&path) {
        Ok(p) => p,
        Err(e) => return fs_errno(e),
    };

    let meta = match KERNEL.vfs.read().stat(&path) {
        Ok(m) => m,
        Err(e) => return fs_errno(e),
    };
    if process.euid() != 0 && process.euid() != meta.uid {
        return abi::errno(abi::EPERM);
    }
    match KERNEL.vfs.read().chmod(path.as_str(), mode as u16 & 0o777) {
        Ok(()) => 0,
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_chown(path_ptr: u32, uid: u32, gid: u32) -> u32 {
    let (process, path) = match current_path(path_ptr) {
        Ok(context) => context,
        Err(e) => return fs_errno(e),
    };
    let (mut uid, mut gid) = (uid, gid);
    let path = match process.follow_path(&path) {
        Ok(p) => p,
        Err(e) => return fs_errno(e),
    };

    if process.euid() != 0 {
        return abi::errno(abi::EPERM);
    }
    let vfs = KERNEL.vfs.read();
    if uid == u32::MAX || gid == u32::MAX {
        let metadata = match vfs.stat(path.as_str()) {
            Ok(metadata) => metadata,
            Err(error) => return fs_errno(error),
        };
        if uid == u32::MAX {
            uid = metadata.uid;
        }
        if gid == u32::MAX {
            gid = metadata.gid;
        }
    }

    match vfs.chown(path.as_str(), uid, gid) {
        Ok(()) => 0,
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_umask(mask: u32) -> u32 {
    with_current_task(|task| Some(task.process.set_umask(mask as u16) as u32))
        .unwrap_or_else(|| abi::errno(abi::EFAULT))
}

pub fn syscall_chdir(path_ptr: u32) -> u32 {
    let (process, path) = match current_path(path_ptr) {
        Ok(context) => context,
        Err(e) => return fs_errno(e),
    };
    let path = match process.follow_path(&path) {
        Ok(p) => p,
        Err(e) => return fs_errno(e),
    };

    if path == "/" {
        process.set_cwd(path);
        return 0;
    }

    match KERNEL.vfs.read().stat(path.as_str()) {
        Ok(metadata) if metadata.is_dir && process.has_permission(&metadata, ACCESS_EXECUTE) => {
            process.set_cwd(path);
            0
        }
        Ok(metadata) if metadata.is_dir => abi::errno(abi::EACCES),
        Ok(_) => abi::errno(abi::ENOTDIR),
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_getcwd(buf_ptr: u32, size: u32) -> u32 {
    let Some((process, buf_ptr, size)) =
        with_current_task(|task| Some((task.process.clone(), buf_ptr, size as usize)))
    else {
        return abi::errno(abi::EFAULT);
    };

    if buf_ptr == 0 || size == 0 {
        return abi::errno(abi::EFAULT);
    }

    let host = process.cwd.lock().clone();
    let root = process.control.lock().root.clone();
    let cwd = if root == "/" {
        host
    } else {
        alloc::format!(
            "/{}",
            host.strip_prefix(&root)
                .unwrap_or("")
                .trim_start_matches('/')
        )
    };
    let bytes = cwd.as_bytes();
    if bytes.len() + 1 > size {
        return abi::errno(abi::EINVAL);
    }

    if user::copy_to_user(
        &process.page_directory,
        buf_ptr,
        bytes.as_ptr(),
        bytes.len() as u32,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    let nul = 0_u8;
    if user::copy_to_user(
        &process.page_directory,
        buf_ptr + bytes.len() as u32,
        &nul as *const u8,
        1,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    (bytes.len() + 1) as u32
}

pub fn syscall_stat(path_ptr: u32, stat_ptr: u32) -> u32 {
    let (process, path) = match current_path(path_ptr) {
        Ok(context) => context,
        Err(e) => return fs_errno(e),
    };
    let path = match process.follow_path(&path) {
        Ok(p) => p,
        Err(e) => return fs_errno(e),
    };

    if stat_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let metadata = match KERNEL.vfs.read().stat(path.as_str()) {
        Ok(metadata) => metadata,
        Err(error) => return fs_errno(error),
    };

    let stat = metadata_to_stat(&metadata);
    if user::write_value(&process.page_directory, stat_ptr, &stat).is_err() {
        return abi::errno(abi::EFAULT);
    }

    0
}

pub fn syscall_lstat(path_ptr: u32, stat_ptr: u32) -> u32 {
    super::filesystem::legacy_lstat(path_ptr, stat_ptr)
}

pub fn syscall_getdents(fd: u32, dirent_ptr: u32, len: u32) -> u32 {
    let Some((process, fd, dirent_ptr, len)) =
        with_current_task(|task| Some((task.process.clone(), fd as i32, dirent_ptr, len as usize)))
    else {
        return abi::errno(abi::EFAULT);
    };

    if dirent_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let directory = match process.get_fd(fd) {
        Some(ProcessDescriptor::Directory(directory)) => directory,
        Some(_) => return abi::errno(abi::ENOTDIR),
        None => return abi::errno(abi::EBADF),
    };

    let mut directory = directory.lock();
    let record_size = core::mem::size_of::<Dirent>();
    let mut written = 0;

    while directory.offset < directory.entries.len() && written + record_size <= len {
        let name = directory.entries[directory.offset].clone();
        let dirent = make_dirent(&directory.path, name.as_str(), directory.offset + 1);
        if user::write_value(
            &process.page_directory,
            dirent_ptr + written as u32,
            &dirent,
        )
        .is_err()
        {
            return abi::errno(abi::EFAULT);
        }

        directory.offset += 1;
        written += record_size;
    }

    written as u32
}

fn remove_path(path_ptr: u32, directory: bool) -> u32 {
    let (process, path) = match current_path(path_ptr) {
        Ok(context) => context,
        Err(e) => return fs_errno(e),
    };
    remove_resolved(&process, &path, directory)
}

pub(super) fn remove_resolved(process: &Process, path: &str, directory: bool) -> u32 {
    if let Err(e) = crate::fs::access::parent_write(&process, &path) {
        return fs_errno(e);
    }
    let vfs = KERNEL.vfs.read();
    match vfs.stat(path) {
        Ok(metadata) if directory && !metadata.is_dir => return abi::errno(abi::ENOTDIR),
        Ok(metadata) if !directory && metadata.is_dir => return abi::errno(abi::EISDIR),
        Ok(_) => {}
        Err(error) => return fs_errno(error),
    }

    match vfs.remove(path) {
        Ok(()) => 0,
        Err(error) => fs_errno(error),
    }
}

fn apply_created_mode(process: &Process, path: &str, mode: u16, directory: bool) {
    let default_mode = if directory { 0o777 } else { 0o666 };
    let mode = process.apply_umask(if mode == 0 {
        default_mode
    } else {
        mode & 0o777
    });
    let _ = KERNEL.vfs.read().chmod(path, mode);
    let _ = KERNEL
        .vfs
        .read()
        .chown(path, process.euid(), process.egid());
}

fn current_path(path_ptr: u32) -> Result<(Arc<Process>, String), FsError> {
    let process =
        with_current_task(|task| Some(task.process.clone())).ok_or(FsError::BadAddress)?;
    let name =
        user::read_process_string(&process, path_ptr, MAX_PATH).ok_or(FsError::BadAddress)?;
    let path = process.resolve_path(&name)?;
    Ok((process, path))
}

fn with_current_task<T>(f: impl FnOnce(&Task) -> Option<T>) -> Option<T> {
    KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        f(&task)
    })
}

fn current_task_id() -> Option<TaskId> {
    KERNEL.with_task_manager(|tm| tm.get_current().map(|task| task.read().id))
}

fn wake_pipe_waiters(waiters: Vec<TaskId>) {
    if waiters.is_empty() {
        return;
    }

    KERNEL.with_task_manager(|tm| {
        for task_id in waiters {
            let _ = tm.wake_task(task_id);
        }
    });
}

fn block_current_and_restart(reason: WaitReason) -> u32 {
    let blocked =
        KERNEL.with_task_manager(|tm| tm.block_current_and_restart_syscall(reason).is_ok());
    if !blocked {
        return abi::errno(abi::EAGAIN);
    }

    task_next();
}

fn seek_for_append(descriptor: &ProcessDescriptor) -> Result<(), FsError> {
    let size = match descriptor.stat() {
        Ok(meta) if !meta.is_dir => meta.size,
        Ok(_) | Err(FsError::Unsupported) => return Ok(()),
        Err(error) => return Err(error),
    };

    match descriptor.seek(size as usize) {
        Ok(_) | Err(FsError::Unsupported) => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn fs_errno(error: FsError) -> u32 {
    let code = match error {
        FsError::NotFound => abi::ENOENT,
        FsError::AlreadyExists => abi::EEXIST,
        FsError::NotADirectory => abi::ENOTDIR,
        FsError::InvalidPath | FsError::InvalidArgument => abi::EINVAL,
        FsError::IoError => abi::EIO,
        FsError::BadAddress => abi::EFAULT,
        FsError::Interrupted => 4,
        FsError::NotSupported => abi::ENOTSUP,
        FsError::BadDescriptor => abi::EBADF,
        FsError::TooManyFiles => abi::EMFILE,
        FsError::Unsupported => abi::ENOTSUP,
        FsError::PermissionDenied => abi::EACCES,
        FsError::NotEmpty => abi::ENOTEMPTY,
        FsError::NoSpace => abi::ENOMEM,
        FsError::IsADirectory => abi::EISDIR,
        FsError::WouldBlock => abi::EAGAIN,
        FsError::BrokenPipe => abi::EPIPE,
        FsError::MessageTooLarge => abi::EMSGSIZE,
    };
    abi::errno(code)
}

fn pipe_fs_error(error: PipeError) -> FsError {
    match error {
        PipeError::WouldBlock => FsError::WouldBlock,
        PipeError::BrokenPipe => FsError::BrokenPipe,
        PipeError::WrongEnd => FsError::InvalidArgument,
        PipeError::MessageTooLarge => FsError::MessageTooLarge,
    }
}

pub(super) fn metadata_to_stat(meta: &crate::fs::FileMetadata) -> FileStat {
    FileStat {
        size: meta.size as u32,
        flags: 0,
        mode: stat_mode(meta),
        uid: meta.uid,
        gid: meta.gid,
        is_dir: meta.is_dir as u32,
    }
}

pub(super) fn stat_mode(meta: &crate::fs::FileMetadata) -> u32 {
    if meta.mode & 0o170000 != 0 {
        return meta.mode as u32;
    }
    let file_type = if meta.is_dir { 0o040000 } else { 0o100000 };
    file_type | meta.mode as u32
}

fn make_dirent(parent: &str, name: &str, index: usize) -> Dirent {
    let mut d_name = [0_u8; 256];
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min(d_name.len() - 1);
    d_name[..copy_len].copy_from_slice(&bytes[..copy_len]);

    Dirent {
        d_ino: index as u32,
        d_off: index as u32,
        d_reclen: core::mem::size_of::<Dirent>() as u16,
        d_type: dirent_type(parent, name),
        d_name,
    }
}

fn dirent_type(parent: &str, name: &str) -> u8 {
    let path = if parent == "/" {
        alloc::format!("/{}", name)
    } else {
        alloc::format!("{}/{}", parent, name)
    };

    match KERNEL.vfs.read().stat(path.as_str()) {
        Ok(metadata) if metadata.is_dir => DT_DIR,
        Ok(_) => DT_REG,
        Err(_) => DT_UNKNOWN,
    }
}

pub(super) fn finish_write(result: u32) -> u32 {
    if result != abi::errno(abi::EPIPE) {
        return result;
    }
    let pid = KERNEL.with_task_manager(|tm| tm.get_current().map(|task| task.read().process.pid));
    if let Some(pid) = pid {
        crate::schedule::task::task_current_set_return_value(result);
        let effect = KERNEL.with_process_manager(|pm| pm.signal(pid, 13));
        if matches!(
            effect,
            Ok(crate::schedule::process_manager::SignalEffect::Delivered
                | crate::schedule::process_manager::SignalEffect::Terminated)
        ) {
            task_next();
        }
    }
    result
}
