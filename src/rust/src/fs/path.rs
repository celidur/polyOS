use super::{FsError, descriptor::ProcessDescriptor};
use crate::schedule::process::Process;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
pub const AT_FDCWD: i32 = -100;
pub fn resolve(base: &str, path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = if path.starts_with('/') {
        Vec::new()
    } else {
        base.split('/').filter(|p| !p.is_empty()).collect()
    };
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    Some(alloc::format!("/{}", parts.join("/")))
}
pub fn resolve_at(process: &Process, fd: i32, path: &str) -> Result<String, FsError> {
    if path.starts_with('/') || fd == AT_FDCWD {
        return process.resolve_path(path).ok_or(FsError::NotFound);
    }
    let base = match process.get_fd(fd) {
        Some(ProcessDescriptor::Directory(d)) => d.lock().path.to_string(),
        Some(_) => return Err(FsError::NotADirectory),
        None => return Err(FsError::BadDescriptor),
    };
    resolve(&base, path).ok_or(FsError::NotFound)
}
