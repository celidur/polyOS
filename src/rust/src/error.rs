use core::fmt;

#[derive(Debug)]
pub enum KernelError {
    Paging,
    Allocation,
    NoTasks,
    Io,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KernelError::{self:?}")
    }
}
