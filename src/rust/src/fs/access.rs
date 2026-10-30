//! Path permission checks shared by filesystem syscall adapters.
use super::{FileMetadata, FsError};
use crate::{kernel::KERNEL, schedule::process::Process};

pub fn search(process: &Process, path: &str) -> Result<(), FsError> {
    let (root, credentials) = {
        let control = process.control.lock();
        (control.root.clone(), control.credentials.clone())
    };
    if path == root {
        return Ok(());
    }
    let suffix = if root == "/" {
        path.strip_prefix('/')
    } else {
        path.strip_prefix(&alloc::format!("{}/", root))
    }
    .ok_or(FsError::PermissionDenied)?;
    let mut parent = root;
    let mut components = suffix.split('/').filter(|part| !part.is_empty()).peekable();
    let vfs = KERNEL.vfs.read();
    while let Some(component) = components.next() {
        let metadata = vfs.lstat(&parent)?;
        if !metadata.is_dir {
            return Err(FsError::NotADirectory);
        }
        if !credentials.permission(&metadata, 1, false) {
            return Err(FsError::PermissionDenied);
        }
        if components.peek().is_some() {
            parent = alloc::format!("{}/{}", parent.trim_end_matches('/'), component);
        }
    }
    Ok(())
}

pub fn parent_write(process: &Process, path: &str) -> Result<(), FsError> {
    search(process, path)?;
    let parent = path
        .rsplit_once('/')
        .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
        .ok_or(FsError::InvalidPath)?;
    let metadata: FileMetadata = KERNEL.vfs.read().lstat(parent)?;
    if !metadata.is_dir {
        return Err(FsError::NotADirectory);
    }
    if !process.has_permission(&metadata, 3) {
        return Err(FsError::PermissionDenied);
    }
    Ok(())
}
