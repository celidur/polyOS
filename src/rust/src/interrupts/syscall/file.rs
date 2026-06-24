use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Mutex;

use crate::{
    constant::MAX_PATH,
    fs::{FileHandle, FsError, Pipe, PipeEnd, PipeError, file::FileStat},
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::{
        process::{
            ACCESS_EXECUTE, ACCESS_READ, ACCESS_WRITE, DirectoryHandle, FD_CLOEXEC, O_NONBLOCK,
            Process, ProcessDescriptor,
        },
        task::{Task, TaskId, WaitReason, task_next},
    },
};

use super::{abi, user};

const O_ACCMODE: u32 = 0x3;
const O_CREAT: u32 = 0x40;
const O_TRUNC: u32 = 0x200;
const O_APPEND: u32 = 0x400;
const SEEK_SET: u32 = 0;
const SEEK_END: u32 = 2;
const F_DUPFD: u32 = 0;
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

pub fn syscall_open(_frame: &InterruptFrame) -> u32 {
    let Some((process, path, flags, mode)) = with_current_task(|task| {
        let path_ptr = task.get_stack_item(0);
        if path_ptr == 0 {
            return None;
        }

        let path = user::read_c_string(task, path_ptr, MAX_PATH)?;
        let path = task.process.resolve_path(path.as_str())?;
        Some((
            task.process.clone(),
            path,
            task.get_stack_item(1),
            task.get_stack_item(2) as u16,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    let mut handle = {
        let vfs = KERNEL.vfs.read();
        match vfs.open(path.as_str()) {
            Ok(handle) => handle,
            Err(_) if flags & O_CREAT != 0 => {
                if let Err(error) = vfs.create(path.as_str(), false) {
                    return fs_errno(error);
                }
                apply_created_mode(&process, path.as_str(), mode, false);

                match vfs.open(path.as_str()) {
                    Ok(handle) => handle,
                    Err(error) => return fs_errno(error),
                }
            }
            Err(error) => {
                if let Some(fd) = insert_directory_fd(&process, path.as_str(), flags) {
                    return fd;
                }

                return fs_errno(error);
            }
        }
    };

    let required_permissions = match open_required_permissions(flags) {
        Ok(permissions) => permissions,
        Err(error) => return fs_errno(error),
    };

    if required_permissions != 0 {
        match handle.ops.stat() {
            Ok(metadata) if !process.has_permission(&metadata, required_permissions) => {
                return abi::errno(abi::EACCES);
            }
            Ok(_) => {}
            Err(error) => return fs_errno(error),
        }
    }

    if flags & O_TRUNC != 0 && flags & O_ACCMODE != 0 {
        if let Err(error) = handle.ops.truncate(0) {
            // Character devices like /dev/null do not need a real truncate.
            // Treat unsupported truncation as a no-op so open() still succeeds.
            if !matches!(error, FsError::Unsupported) {
                return fs_errno(error);
            }
        }
    }

    insert_file_fd(&process, handle, flags)
}

pub fn syscall_read(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, buf_ptr, len)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
            task.get_stack_item(2) as usize,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    if buf_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let descriptor = match process.get_fd(fd) {
        Some(descriptor) => descriptor,
        None => return abi::errno(abi::EBADF),
    };

    if let ProcessDescriptor::Pipe { pipe, end } = &descriptor {
        let result = syscall_pipe_read(&process, fd, pipe.clone(), *end, buf_ptr, len);
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
    let read = match descriptor.read(data.as_mut_slice()) {
        Ok(read) => read,
        Err(error) => return fs_errno(error),
    };

    if read != 0
        && user::copy_to_user(&process.page_directory, buf_ptr, data.as_ptr(), read as u32).is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    read as u32
}

pub fn syscall_write(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, ptr, len)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
            task.get_stack_item(2) as usize,
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

    let Some(descriptor) = process.get_fd(fd) else {
        return abi::errno(abi::EBADF);
    };

    if let ProcessDescriptor::Pipe { pipe, end } = &descriptor {
        let result = syscall_pipe_write(&process, fd, pipe.clone(), *end, data.as_slice());
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
                let waiters = if read > 0 {
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
                let waiters = if written > 0 {
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

pub fn syscall_lseek(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, offset, whence)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
            task.get_stack_item(2),
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    let Some(descriptor) = process.get_fd(fd) else {
        return abi::errno(abi::EBADF);
    };

    let position = match whence {
        SEEK_SET => offset as i32,
        SEEK_END => match descriptor.stat() {
            Ok(meta) => meta.size as i32 + offset as i32,
            Err(error) => return fs_errno(error),
        },
        _ => return abi::errno(abi::EINVAL),
    };

    if position < 0 {
        return abi::errno(abi::EINVAL);
    }

    match descriptor.seek(position as usize) {
        Ok(pos) => pos as u32,
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_fstat(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, stat_ptr)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
        ))
    }) else {
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

pub fn syscall_close(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd)) =
        with_current_task(|task| Some((task.process.clone(), task.get_stack_item(0) as i32)))
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

pub fn syscall_dup(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd)) =
        with_current_task(|task| Some((task.process.clone(), task.get_stack_item(0) as i32)))
    else {
        return abi::errno(abi::EFAULT);
    };

    match process.duplicate_fd(fd) {
        Ok(new_fd) => new_fd as u32,
        Err(_) => abi::errno(abi::EBADF),
    }
}

pub fn syscall_dup2(_frame: &InterruptFrame) -> u32 {
    let Some((process, old_fd, new_fd)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1) as i32,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    match process.duplicate_fd_to(old_fd, new_fd) {
        Ok(fd) => fd as u32,
        Err(_) => abi::errno(abi::EBADF),
    }
}

pub fn syscall_fcntl(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, cmd, arg)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
            task.get_stack_item(2),
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    match cmd {
        F_DUPFD => match process.duplicate_fd_from(fd, arg as i32) {
            Ok(new_fd) => new_fd as u32,
            Err(_) => abi::errno(abi::EBADF),
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

pub fn syscall_pipe(_frame: &InterruptFrame) -> u32 {
    let Some((process, pipefd_ptr)) =
        with_current_task(|task| Some((task.process.clone(), task.get_stack_item(0))))
    else {
        return abi::errno(abi::EFAULT);
    };

    if pipefd_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let pipe = Arc::new(Mutex::new(Pipe::new()));
    let read_fd = match process.insert_fd(ProcessDescriptor::Pipe {
        pipe: pipe.clone(),
        end: PipeEnd::Read,
    }) {
        Ok(fd) => fd,
        Err(_) => return abi::errno(abi::EMFILE),
    };

    let write_fd = match process.insert_fd(ProcessDescriptor::Pipe {
        pipe,
        end: PipeEnd::Write,
    }) {
        Ok(fd) => fd,
        Err(_) => {
            if let Some(descriptor) = process.remove_fd(read_fd) {
                descriptor.close();
            }
            return abi::errno(abi::EMFILE);
        }
    };

    let pipefd = [read_fd, write_fd];
    if user::write_value(&process.page_directory, pipefd_ptr, &pipefd).is_err() {
        if let Some(descriptor) = process.remove_fd(read_fd) {
            descriptor.close();
        }
        if let Some(descriptor) = process.remove_fd(write_fd) {
            descriptor.close();
        }
        return abi::errno(abi::EFAULT);
    }

    0
}

pub fn syscall_unlink(_frame: &InterruptFrame) -> u32 {
    remove_path(false)
}

pub fn syscall_rmdir(_frame: &InterruptFrame) -> u32 {
    remove_path(true)
}

pub fn syscall_mkdir(_frame: &InterruptFrame) -> u32 {
    let Some((process, path, mode)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            current_resolved_path_for_task(task, 0)?,
            task.get_stack_item(1) as u16,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    match KERNEL.vfs.read().create(path.as_str(), true) {
        Ok(()) => {
            apply_created_mode(&process, path.as_str(), mode, true);
            0
        }
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_chmod(_frame: &InterruptFrame) -> u32 {
    let Some((path, mode)) = with_current_task(|task| {
        Some((
            current_resolved_path_for_task(task, 0)?,
            task.get_stack_item(1) as u16,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    match KERNEL.vfs.read().chmod(path.as_str(), mode & 0o777) {
        Ok(()) => 0,
        Err(error) => fs_errno(error),
    }
}

pub fn syscall_chown(_frame: &InterruptFrame) -> u32 {
    let Some((path, mut uid, mut gid)) = with_current_task(|task| {
        Some((
            current_resolved_path_for_task(task, 0)?,
            task.get_stack_item(1),
            task.get_stack_item(2),
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

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

pub fn syscall_umask(_frame: &InterruptFrame) -> u32 {
    with_current_task(|task| Some(task.process.set_umask(task.get_stack_item(0) as u16) as u32))
        .unwrap_or_else(|| abi::errno(abi::EFAULT))
}

pub fn syscall_chdir(_frame: &InterruptFrame) -> u32 {
    let Some((process, path)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            current_resolved_path_for_task(task, 0)?,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
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

pub fn syscall_getcwd(_frame: &InterruptFrame) -> u32 {
    let Some((process, buf_ptr, size)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0),
            task.get_stack_item(1) as usize,
        ))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    if buf_ptr == 0 || size == 0 {
        return abi::errno(abi::EFAULT);
    }

    let cwd = process.cwd.lock().clone();
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

pub fn syscall_stat(_frame: &InterruptFrame) -> u32 {
    let Some((process, path, stat_ptr)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            current_resolved_path_for_task(task, 0)?,
            task.get_stack_item(1),
        ))
    }) else {
        return abi::errno(abi::EFAULT);
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

pub fn syscall_lstat(frame: &InterruptFrame) -> u32 {
    syscall_stat(frame)
}

pub fn syscall_getdents(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd, dirent_ptr, len)) = with_current_task(|task| {
        Some((
            task.process.clone(),
            task.get_stack_item(0) as i32,
            task.get_stack_item(1),
            task.get_stack_item(2) as usize,
        ))
    }) else {
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

fn insert_file_fd(process: &Process, handle: FileHandle, flags: u32) -> u32 {
    match process
        .insert_fd_with_status_flags(ProcessDescriptor::File(Arc::new(Mutex::new(handle))), flags)
    {
        Ok(fd) => fd as u32,
        Err(_) => abi::errno(abi::EMFILE),
    }
}

fn insert_directory_fd(process: &Process, path: &str, flags: u32) -> Option<u32> {
    let vfs = KERNEL.vfs.read();
    let metadata = vfs.stat(path).ok()?;
    if !metadata.is_dir {
        return None;
    }

    match flags & O_ACCMODE {
        0 => {}
        1 | 2 => return Some(abi::errno(abi::EISDIR)),
        _ => return Some(abi::errno(abi::EINVAL)),
    }

    if flags & O_TRUNC != 0 {
        return Some(abi::errno(abi::EISDIR));
    }

    if !process.has_permission(&metadata, ACCESS_READ) {
        return Some(abi::errno(abi::EACCES));
    }

    let entries = vfs.read_dir(path).ok()?;
    let directory = DirectoryHandle {
        path: path.into(),
        entries,
        offset: 0,
        metadata,
    };

    match process.insert_fd_with_status_flags(
        ProcessDescriptor::Directory(Arc::new(Mutex::new(directory))),
        flags,
    ) {
        Ok(fd) => Some(fd as u32),
        Err(_) => Some(abi::errno(abi::EMFILE)),
    }
}

fn remove_path(directory: bool) -> u32 {
    let Some(path) = current_resolved_path(0) else {
        return abi::errno(abi::EFAULT);
    };

    let vfs = KERNEL.vfs.read();
    match vfs.stat(path.as_str()) {
        Ok(metadata) if directory && !metadata.is_dir => return abi::errno(abi::ENOTDIR),
        Ok(metadata) if !directory && metadata.is_dir => return abi::errno(abi::EISDIR),
        Ok(_) => {}
        Err(error) => return fs_errno(error),
    }

    match vfs.remove(path.as_str()) {
        Ok(()) => 0,
        Err(error) => fs_errno(error),
    }
}

fn open_required_permissions(flags: u32) -> Result<u16, FsError> {
    let mut permissions = match flags & O_ACCMODE {
        0 => ACCESS_READ,
        1 => ACCESS_WRITE,
        2 => ACCESS_READ | ACCESS_WRITE,
        _ => return Err(FsError::InvalidArgument),
    };

    if flags & O_TRUNC != 0 {
        permissions |= ACCESS_WRITE;
    }

    Ok(permissions)
}

fn apply_created_mode(process: &Process, path: &str, mode: u16, directory: bool) {
    let default_mode = if directory { 0o777 } else { 0o666 };
    let mode = process.apply_umask(if mode == 0 {
        default_mode
    } else {
        mode & 0o777
    });
    let _ = KERNEL.vfs.read().chmod(path, mode);
}

fn current_resolved_path(stack_index: u32) -> Option<String> {
    with_current_task(|task| current_resolved_path_for_task(task, stack_index))
}

fn current_resolved_path_for_task(task: &Task, stack_index: u32) -> Option<String> {
    let path_ptr = task.get_stack_item(stack_index as usize);
    if path_ptr == 0 {
        return None;
    }

    let path = user::read_c_string(task, path_ptr, MAX_PATH)?;
    task.process.resolve_path(path.as_str())
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

fn fs_errno(error: FsError) -> u32 {
    let code = match error {
        FsError::NotFound => abi::ENOENT,
        FsError::AlreadyExists => abi::EEXIST,
        FsError::NotADirectory => abi::ENOTDIR,
        FsError::InvalidPath | FsError::InvalidArgument => abi::EINVAL,
        FsError::IoError => abi::EIO,
        FsError::Unsupported => abi::ENOTSUP,
        FsError::PermissionDenied => abi::EACCES,
        FsError::NotEmpty => abi::ENOTEMPTY,
        FsError::NoSpace => abi::ENOMEM,
        FsError::IsADirectory => abi::EISDIR,
        FsError::WouldBlock => abi::EAGAIN,
        FsError::BrokenPipe => abi::EPIPE,
    };
    abi::errno(code)
}

fn pipe_fs_error(error: PipeError) -> FsError {
    match error {
        PipeError::WouldBlock => FsError::WouldBlock,
        PipeError::BrokenPipe => FsError::BrokenPipe,
        PipeError::WrongEnd => FsError::InvalidArgument,
    }
}

fn metadata_to_stat(meta: &crate::fs::FileMetadata) -> FileStat {
    FileStat {
        size: meta.size as u32,
        flags: 0,
        mode: stat_mode(meta),
        uid: meta.uid,
        gid: meta.gid,
        is_dir: meta.is_dir as u32,
    }
}

fn stat_mode(meta: &crate::fs::FileMetadata) -> u32 {
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
