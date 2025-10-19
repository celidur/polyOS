mod dispatcher;
mod file;
mod heap;
mod io;
mod misc;
mod process;
mod register;
mod types;

pub use dispatcher::{syscall_handle, syscall_init};
pub use types::SyscallId;
