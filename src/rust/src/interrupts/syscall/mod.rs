mod abi;
mod dispatcher;
mod file;
mod filesystem;
mod heap;
mod io;
mod ipc;
mod misc;
mod network;
mod process;
mod register;
mod signal;
mod sync;
mod thread;
mod types;
use crate::memory::user;

pub use dispatcher::{syscall_handle, syscall_init};
pub use types::SyscallId;

mod inspection;
