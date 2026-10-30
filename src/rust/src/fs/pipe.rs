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
    MessageTooLarge,
}

pub struct Pipe {
    packet_sizes: Option<VecDeque<usize>>,
    buffer: VecDeque<u8>,
    readers: usize,
    writers: usize,
    read_waiters: VecDeque<TaskId>,
    write_waiters: VecDeque<TaskId>,
}

impl Pipe {
    pub fn new() -> Self {
        Self {
            packet_sizes: None,
            buffer: VecDeque::new(),
            readers: 1,
            writers: 1,
            read_waiters: VecDeque::new(),
            write_waiters: VecDeque::new(),
        }
    }
    pub fn packets() -> Self {
        Self {
            packet_sizes: Some(VecDeque::new()),
            ..Self::new()
        }
    }
    pub fn is_packet(&self) -> bool {
        self.packet_sizes.is_some()
    }

    pub fn poll(&self, end: PipeEnd, events: u16) -> u16 {
        match end {
            PipeEnd::Read => {
                (if !self.buffer.is_empty()
                    || self.packet_sizes.as_ref().is_some_and(|q| !q.is_empty())
                    || self.writers == 0
                {
                    events & 1
                } else {
                    0
                }) | (if self.writers == 0 { 16 } else { 0 })
            }
            PipeEnd::Write => {
                (if self.buffer.len() < PIPE_CAPACITY
                    && self.packet_sizes.as_ref().is_none_or(|q| q.len() < 128)
                {
                    events & 4
                } else {
                    0
                }) | (if self.readers == 0 { 8 } else { 0 })
            }
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

        if self.buffer.is_empty() && self.packet_sizes.as_ref().is_none_or(|q| q.is_empty()) {
            return if self.writers == 0 {
                Ok(0)
            } else {
                Err(PipeError::WouldBlock)
            };
        }

        let packet = self.packet_sizes.as_mut().and_then(|q| q.pop_front());
        let limit = packet.unwrap_or(out.len()).min(out.len());
        let mut read = 0;
        while read < limit {
            let Some(byte) = self.buffer.pop_front() else {
                break;
            };

            out[read] = byte;
            read += 1;
        }
        if let Some(size) = packet {
            for _ in read..size {
                self.buffer.pop_front();
            }
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

        if let Some(queue) = &self.packet_sizes {
            if input.len() > PIPE_CAPACITY {
                return Err(PipeError::MessageTooLarge);
            }
            if queue.len() >= 128 {
                return Err(PipeError::WouldBlock);
            }
        }
        if input.is_empty() && !self.is_packet() {
            return Ok(0);
        }

        let available = PIPE_CAPACITY.saturating_sub(self.buffer.len());
        if (available == 0 && !input.is_empty())
            || (input.len() <= PIPE_CAPACITY && available < input.len())
        {
            return Err(PipeError::WouldBlock);
        }

        let to_write = available.min(input.len());
        self.buffer.extend(input[..to_write].iter().copied());
        if let Some(queue) = &mut self.packet_sizes {
            queue.push_back(to_write);
        }
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
