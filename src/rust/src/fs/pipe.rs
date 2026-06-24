use alloc::{collections::VecDeque, vec::Vec};

use crate::schedule::task::TaskId;

pub const PIPE_CAPACITY: usize = 4096;

#[derive(Clone, Copy)]
pub enum PipeEnd {
    Read,
    Write,
}

#[derive(Debug)]
pub enum PipeError {
    WouldBlock,
    BrokenPipe,
    WrongEnd,
}

pub struct Pipe {
    buffer: VecDeque<u8>,
    readers: usize,
    writers: usize,
    read_waiters: VecDeque<TaskId>,
    write_waiters: VecDeque<TaskId>,
}

impl Pipe {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            readers: 1,
            writers: 1,
            read_waiters: VecDeque::new(),
            write_waiters: VecDeque::new(),
        }
    }

    pub fn id(&self) -> usize {
        self as *const Self as usize
    }

    pub fn clone_end(&mut self, end: PipeEnd) {
        match end {
            PipeEnd::Read => self.readers = self.readers.saturating_add(1),
            PipeEnd::Write => self.writers = self.writers.saturating_add(1),
        }
    }

    pub fn close_end(&mut self, end: PipeEnd) -> Vec<TaskId> {
        match end {
            PipeEnd::Read => {
                self.readers = self.readers.saturating_sub(1);
                self.take_write_waiters()
            }
            PipeEnd::Write => {
                self.writers = self.writers.saturating_sub(1);
                self.take_read_waiters()
            }
        }
    }

    pub fn read(&mut self, end: PipeEnd, out: &mut [u8]) -> Result<usize, PipeError> {
        if !matches!(end, PipeEnd::Read) {
            return Err(PipeError::WrongEnd);
        }

        if out.is_empty() {
            return Ok(0);
        }

        if self.buffer.is_empty() {
            return if self.writers == 0 {
                Ok(0)
            } else {
                Err(PipeError::WouldBlock)
            };
        }

        let mut read = 0;
        while read < out.len() {
            let Some(byte) = self.buffer.pop_front() else {
                break;
            };

            out[read] = byte;
            read += 1;
        }

        Ok(read)
    }

    pub fn write(&mut self, end: PipeEnd, input: &[u8]) -> Result<usize, PipeError> {
        if !matches!(end, PipeEnd::Write) {
            return Err(PipeError::WrongEnd);
        }

        if self.readers == 0 {
            return Err(PipeError::BrokenPipe);
        }

        if input.is_empty() {
            return Ok(0);
        }

        let available = PIPE_CAPACITY.saturating_sub(self.buffer.len());
        if available == 0 {
            return Err(PipeError::WouldBlock);
        }

        let to_write = available.min(input.len());
        self.buffer.extend(input[..to_write].iter().copied());
        Ok(to_write)
    }

    pub fn add_read_waiter(&mut self, task_id: TaskId) {
        if !self.read_waiters.contains(&task_id) {
            self.read_waiters.push_back(task_id);
        }
    }

    pub fn add_write_waiter(&mut self, task_id: TaskId) {
        if !self.write_waiters.contains(&task_id) {
            self.write_waiters.push_back(task_id);
        }
    }

    pub fn take_read_waiters(&mut self) -> Vec<TaskId> {
        self.read_waiters.drain(..).collect()
    }

    pub fn take_write_waiters(&mut self) -> Vec<TaskId> {
        self.write_waiters.drain(..).collect()
    }
}
