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
        return process.resolve_path(path);
    }
    let base = match process.get_fd(fd) {
        Some(ProcessDescriptor::Directory(d)) => d.lock().path.to_string(),
        Some(_) => return Err(FsError::NotADirectory),
        None => return Err(FsError::BadDescriptor),
    };
    process.resolve_from(&base, path)
}

/// Walk before simplifying dot components. Callers must release scheduler/task
/// locks before entering filesystem policy (procfs may inspect those tasks).
pub fn resolve_for(process: &Process, base: &str, path: &str) -> Result<String, FsError> {
    if path.is_empty() {
        return Err(FsError::NotFound);
    }
    let (root, credentials) = {
        let control = process.control.lock();
        (control.root.clone(), control.credentials.clone())
    };
    let mut current = if path.starts_with('/') {
        root.clone()
    } else {
        base.to_string()
    };
    if root != "/" && current != root && !current.starts_with(&alloc::format!("{}/", root)) {
        return Err(FsError::NotFound);
    }
    let vfs = crate::kernel::KERNEL.vfs.read();
    let directory = |name: &str| -> Result<String, FsError> {
        let resolved = vfs.follow_in(&root, name)?;
        let metadata = vfs.lstat(&resolved)?;
        if !metadata.is_dir {
            return Err(FsError::NotADirectory);
        }
        if !credentials.permission(&metadata, 1, false) {
            return Err(FsError::PermissionDenied);
        }
        Ok(resolved)
    };
    for component in path.split('/').filter(|s| !s.is_empty()) {
        current = directory(&current)?;
        match component {
            "." => {}
            ".." if current == root => {}
            ".." => {
                current = current
                    .rsplit_once('/')
                    .map(|(p, _)| if p.is_empty() { "/" } else { p })
                    .unwrap_or(&root)
                    .to_string();
            }
            name => current = alloc::format!("{}/{}", current.trim_end_matches('/'), name),
        }
    }
    if path.ends_with('/') {
        current = directory(&current)?;
    }
    Ok(current)
}

pub fn resolve_root(root: &str, base: &str, path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let base = if path.starts_with('/') {
        ""
    } else if root == "/" {
        base.trim_start_matches('/')
    } else if base == root {
        ""
    } else {
        base.strip_prefix(&alloc::format!("{}/", root))?
    };
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let suffix = parts.join("/");
    Some(if suffix.is_empty() {
        root.to_string()
    } else {
        alloc::format!("{}/{}", root.trim_end_matches('/'), suffix)
    })
}
