use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    constant::MAX_PATH,
    fs::{FileHandle, Pipe, PipeEnd, file::FileStat},
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::{
        process::{Process, ProcessDescriptor},
        task::Task,
    },
};

use super::{abi, user};

const O_CREAT: u32 = 0x40;
const SEEK_SET: u32 = 0;

pub fn syscall_open(_frame: &InterruptFrame) -> u32 {
    let Some((process, path, flags)) = with_current_task(|task| {
        let path_ptr = task.get_stack_item(0);
        if path_ptr == 0 {
            return None;
        }

        let path = user::read_c_string(task, path_ptr, MAX_PATH)?;
        Some((task.process.clone(), path, task.get_stack_item(1)))
    }) else {
        return syscall_error();
    };

    let handle = {
        let vfs = KERNEL.vfs.read();
        match vfs.open(path.as_str()) {
            Ok(handle) => handle,
            Err(_) if flags & O_CREAT != 0 => {
                if vfs.create(path.as_str(), false).is_err() {
                    return syscall_error();
                }

                match vfs.open(path.as_str()) {
                    Ok(handle) => handle,
                    Err(_) => return syscall_error(),
                }
            }
            Err(_) => return syscall_error(),
        }
    };

    insert_file_fd(&process, handle)
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
        return syscall_error();
    };

    if buf_ptr == 0 {
        return syscall_error();
    }

    let descriptor = match process.get_fd(fd) {
        Some(descriptor) => descriptor,
        None => return syscall_error(),
    };

    let mut data = vec![0; len];
    let read = match descriptor.read(data.as_mut_slice()) {
        Ok(read) => read,
        Err(_) => return syscall_error(),
    };

    if read != 0
        && user::copy_to_user(&process.page_directory, buf_ptr, data.as_ptr(), read as u32).is_err()
    {
        return syscall_error();
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
        return syscall_error();
    };

    if ptr == 0 {
        return syscall_error();
    }

    let mut data = vec![0; len];
    if len != 0
        && user::copy_from_user(&process.page_directory, ptr, data.as_mut_ptr(), len as u32)
            .is_err()
    {
        return syscall_error();
    }

    match process
        .get_fd(fd)
        .and_then(|descriptor| descriptor.write(data.as_slice()).ok())
    {
        Some(written) => written as u32,
        None => syscall_error(),
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
        return syscall_error();
    };

    if whence != SEEK_SET {
        return syscall_error();
    }

    let Some(descriptor) = process.get_fd(fd) else {
        return syscall_error();
    };

    match descriptor.seek(offset as usize) {
        Ok(pos) => pos as u32,
        Err(_) => syscall_error(),
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
        return syscall_error();
    };

    if stat_ptr == 0 {
        return syscall_error();
    }

    let Some(descriptor) = process.get_fd(fd) else {
        return syscall_error();
    };

    let meta = match descriptor.stat() {
        Ok(meta) => meta,
        Err(_) => return syscall_error(),
    };

    let stat = FileStat {
        size: meta.size as u32,
        flags: 0,
    };

    if user::write_value(&process.page_directory, stat_ptr, &stat).is_err() {
        return syscall_error();
    }

    0
}

pub fn syscall_close(_frame: &InterruptFrame) -> u32 {
    let Some((process, fd)) =
        with_current_task(|task| Some((task.process.clone(), task.get_stack_item(0) as i32)))
    else {
        return syscall_error();
    };

    match process.remove_fd(fd) {
        Some(descriptor) => {
            descriptor.close();
            0
        }
        None => syscall_error(),
    }
}

pub fn syscall_pipe(_frame: &InterruptFrame) -> u32 {
    let Some((process, pipefd_ptr)) =
        with_current_task(|task| Some((task.process.clone(), task.get_stack_item(0))))
    else {
        return syscall_error();
    };

    if pipefd_ptr == 0 {
        return syscall_error();
    }

    let pipe = Arc::new(Mutex::new(Pipe::new()));
    let read_fd = match process.insert_fd(ProcessDescriptor::Pipe {
        pipe: pipe.clone(),
        end: PipeEnd::Read,
    }) {
        Ok(fd) => fd,
        Err(_) => return syscall_error(),
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
            return syscall_error();
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
        return syscall_error();
    }

    0
}

fn insert_file_fd(process: &Process, handle: FileHandle) -> u32 {
    match process.insert_fd(ProcessDescriptor::File(Arc::new(Mutex::new(handle)))) {
        Ok(fd) => fd as u32,
        Err(_) => syscall_error(),
    }
}

fn with_current_task<T>(f: impl FnOnce(&Task) -> Option<T>) -> Option<T> {
    KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        f(&task)
    })
}

fn syscall_error() -> u32 {
    abi::error()
}
