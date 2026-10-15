//! Open descriptors and endpoint lifetime; processes only own descriptor tables.
use super::{FileHandle, FileMetadata, FsError, Pipe, PipeEnd, PipeError};
use crate::{kernel::KERNEL, memory::PageDirectory, schedule::task::TaskId};
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Mutex;
const OPEN_STATUS_FLAGS: u32 = 3 | 0x400 | 0x800;
#[derive(Clone)]
pub enum ProcessDescriptor {
    File(Arc<Mutex<FileHandle>>),
    Directory(Arc<Mutex<DirectoryHandle>>),
    Socket(Arc<Mutex<SocketHandle>>),
    LocalSocket {
        rx: Arc<Mutex<Pipe>>,
        tx: Arc<Mutex<Pipe>>,
    },
    Pipe {
        pipe: Arc<Mutex<Pipe>>,
        end: PipeEnd,
    },
}

#[derive(Clone)]
pub struct ProcessFd {
    pub(crate) descriptor: ProcessDescriptor,
    pub(crate) flags: u32,
    pub(crate) status_flags: Arc<Mutex<u32>>,
}

impl ProcessFd {
    pub(crate) fn new_with_status_flags(descriptor: ProcessDescriptor, status_flags: u32) -> Self {
        Self {
            descriptor,
            flags: 0,
            status_flags: Arc::new(Mutex::new(status_flags & OPEN_STATUS_FLAGS)),
        }
    }

    pub(crate) fn duplicate_for_fork(&self) -> Self {
        Self {
            descriptor: self.descriptor.duplicate_for_fd_table(),
            flags: self.flags,
            status_flags: self.status_flags.clone(),
        }
    }

    pub(crate) fn duplicate_for_dup(&self) -> Self {
        Self {
            descriptor: self.descriptor.duplicate_for_fd_table(),
            flags: 0,
            status_flags: self.status_flags.clone(),
        }
    }

    pub(crate) fn close(self) {
        self.descriptor.close();
    }
}

#[derive(Clone)]
pub struct DirectoryHandle {
    pub path: String,
    pub entries: Vec<String>,
    pub offset: usize,
    pub metadata: FileMetadata,
}

pub struct SocketHandle {
    id: u32,
    refs: usize,
}

impl SocketHandle {
    pub fn new(id: u32) -> Self {
        Self { id, refs: 1 }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    fn duplicate(&mut self) {
        self.refs = self.refs.saturating_add(1);
    }

    pub(crate) fn close(&mut self) -> Option<u32> {
        self.refs = self.refs.saturating_sub(1);
        (self.refs == 0).then_some(self.id)
    }
}

impl ProcessDescriptor {
    pub fn position(&self) -> Result<usize, FsError> {
        match self {
            Self::File(f) => f.lock().ops.position(),
            Self::Directory(d) => Ok(d.lock().offset),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn pipe_endpoint(&self, write: bool) -> Option<(Arc<Mutex<Pipe>>, PipeEnd)> {
        match self {
            Self::Pipe { pipe, end } => Some((pipe.clone(), *end)),
            Self::LocalSocket { rx, tx } => Some((
                if write { tx.clone() } else { rx.clone() },
                if write { PipeEnd::Write } else { PipeEnd::Read },
            )),
            _ => None,
        }
    }
    pub fn poll(&self, events: u16) -> u16 {
        match self {
            Self::File(file) => file.lock().ops.poll(events),
            Self::Directory(_) => events & 1,
            Self::Socket(socket) => crate::net::socket_poll(socket.lock().id(), events),
            Self::Pipe { pipe, end } => pipe.lock().poll(*end, events),
            Self::LocalSocket { rx, tx } => {
                rx.lock().poll(PipeEnd::Read, events) | tx.lock().poll(PipeEnd::Write, events)
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.read(buf),
            Self::Directory(_) => Err(FsError::IsADirectory),
            Self::Pipe { pipe, end } => {
                let (result, waiters) = {
                    let mut pipe = pipe.lock();
                    let result = pipe.read(*end, buf);
                    let waiters = if matches!(result, Ok(read) if read > 0) {
                        pipe.take_write_waiters()
                    } else {
                        Vec::new()
                    };
                    (result, waiters)
                };
                wake_tasks(waiters);
                result.map_err(pipe_error)
            }
            Self::LocalSocket { rx, .. } => Self::Pipe {
                pipe: rx.clone(),
                end: PipeEnd::Read,
            }
            .read(buf),
            Self::Socket(_) => Err(FsError::Unsupported),
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.write(buf),
            Self::Directory(_) => Err(FsError::IsADirectory),
            Self::Pipe { pipe, end } => {
                let (result, waiters) = {
                    let mut pipe = pipe.lock();
                    let result = pipe.write(*end, buf);
                    let waiters = if matches!(result, Ok(written) if written > 0) {
                        pipe.take_read_waiters()
                    } else {
                        Vec::new()
                    };
                    (result, waiters)
                };
                wake_tasks(waiters);
                result.map_err(pipe_error)
            }
            Self::LocalSocket { tx, .. } => Self::Pipe {
                pipe: tx.clone(),
                end: PipeEnd::Write,
            }
            .write(buf),
            Self::Socket(_) => Err(FsError::Unsupported),
        }
    }

    pub fn seek(&self, pos: usize) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.seek(pos),
            Self::Directory(directory) => {
                let mut directory = directory.lock();
                directory.offset = pos.min(directory.entries.len());
                Ok(directory.offset)
            }
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn ioctl(&self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        match self {
            Self::File(file) => file.lock().ops.ioctl(request, arg, directory),
            Self::Directory(_) => Err(FsError::Unsupported),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn stat(&self) -> Result<FileMetadata, FsError> {
        match self {
            Self::File(file) => file.lock().ops.stat(),
            Self::Directory(directory) => Ok(directory.lock().metadata.clone()),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn duplicate_for_fd_table(&self) -> Self {
        match self {
            Self::LocalSocket { rx, tx } => {
                rx.lock().clone_end(PipeEnd::Read);
                tx.lock().clone_end(PipeEnd::Write);
                Self::LocalSocket {
                    rx: rx.clone(),
                    tx: tx.clone(),
                }
            }
            Self::File(file) => Self::File(file.clone()),
            Self::Directory(directory) => Self::Directory(directory.clone()),
            Self::Socket(socket) => {
                socket.lock().duplicate();
                Self::Socket(socket.clone())
            }
            Self::Pipe { pipe, end } => {
                pipe.lock().clone_end(*end);
                Self::Pipe {
                    pipe: pipe.clone(),
                    end: *end,
                }
            }
        }
    }

    pub fn close(self) {
        match self {
            Self::LocalSocket { rx, tx } => {
                let a = rx.lock().close_end(PipeEnd::Read);
                let b = tx.lock().close_end(PipeEnd::Write);
                wake_tasks(a);
                wake_tasks(b);
            }
            Self::File(_) => {}
            Self::Directory(_) => {}
            Self::Socket(socket) => {
                if let Some(socket_id) = socket.lock().close() {
                    let _ = crate::net::socket_close(socket_id);
                }
            }
            Self::Pipe { pipe, end } => {
                let waiters = pipe.lock().close_end(end);
                wake_tasks(waiters);
            }
        }
    }
}

fn wake_tasks(waiters: Vec<TaskId>) {
    if waiters.is_empty() {
        return;
    }

    KERNEL.with_task_manager(|tm| {
        for task_id in waiters {
            let _ = tm.wake_task(task_id);
        }
    });
}

fn pipe_error(error: PipeError) -> FsError {
    match error {
        PipeError::WouldBlock => FsError::WouldBlock,
        PipeError::BrokenPipe => FsError::BrokenPipe,
        PipeError::WrongEnd => FsError::InvalidArgument,
    }
}
