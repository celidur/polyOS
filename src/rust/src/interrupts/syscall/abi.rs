#![allow(dead_code)]

pub const EPERM: i32 = 1;
pub const ENOENT: i32 = 2;
pub const ESRCH: i32 = 3;
pub const EIO: i32 = 5;
pub const EBADF: i32 = 9;
pub const ECHILD: i32 = 10;
pub const EAGAIN: i32 = 11;
pub const ENOMEM: i32 = 12;
pub const EACCES: i32 = 13;
pub const EFAULT: i32 = 14;
pub const EEXIST: i32 = 17;
pub const ENODEV: i32 = 19;
pub const ENOTDIR: i32 = 20;
pub const EISDIR: i32 = 21;
pub const EINVAL: i32 = 22;
pub const EMFILE: i32 = 24;
pub const ENOTTY: i32 = 25;
pub const EPIPE: i32 = 32;
pub const ENOSYS: i32 = 38;
pub const ENOTEMPTY: i32 = 39;
pub const EMSGSIZE: i32 = 90;
pub const ENOTSUP: i32 = 95;
pub const ENETDOWN: i32 = 100;
pub const ENOTCONN: i32 = 107;

pub const ERROR: u32 = (-EINVAL) as u32;
pub const WAIT_TIMEOUT: u32 = (-EAGAIN) as u32;

#[inline]
pub const fn error() -> u32 {
    ERROR
}

#[inline]
pub const fn errno(code: i32) -> u32 {
    (-code) as u32
}
