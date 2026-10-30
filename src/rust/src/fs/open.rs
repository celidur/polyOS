//! Shared open policy for open/openat; no user pointers or machine registers.
use super::{
    FsError,
    descriptor::{DirectoryHandle, ProcessDescriptor},
};
use crate::{kernel::KERNEL, schedule::process::Process};
use alloc::sync::Arc;
use spin::Mutex;
pub fn open(process: &Process, path: &str, flags: u32, mode: u16) -> Result<i32, FsError> {
    const CREAT: u32 = 0x40;
    const EXCL: u32 = 0x80;
    const TRUNC: u32 = 0x200;
    const DIRECTORY: u32 = 0x10000;
    if flags & 3 == 3
        || flags & !(3 | CREAT | EXCL | TRUNC | 0x400 | 0x800 | DIRECTORY | 0x80000) != 0
    {
        return Err(FsError::InvalidArgument);
    }
    let resolved = process.follow_path(path)?;
    let path = resolved.as_str();
    let vfs = KERNEL.vfs.read();
    let mut created = false;
    let metadata = match vfs.lstat(path) {
        Ok(meta) => {
            if flags & (CREAT | EXCL) == CREAT | EXCL {
                return Err(FsError::AlreadyExists);
            }
            meta
        }
        Err(FsError::NotFound) if flags & CREAT != 0 => {
            if flags & DIRECTORY != 0 {
                return Err(FsError::NotADirectory);
            }
            super::access::parent_write(process, path)?;
            vfs.create(path, false)?;
            created = true;
            match vfs.chown(path, process.euid(), process.egid()) {
                Ok(()) | Err(FsError::Unsupported) => {}
                Err(e) => return Err(e),
            }
            match vfs.chmod(path, process.apply_umask(mode)) {
                Ok(()) | Err(FsError::Unsupported) => {}
                Err(e) => return Err(e),
            }
            vfs.stat(path)?
        }
        Err(e) => return Err(e),
    };
    if flags & DIRECTORY != 0 && !metadata.is_dir {
        return Err(FsError::NotADirectory);
    }
    let permissions = match flags & 3 {
        0 => 4,
        1 => 2,
        _ => 6,
    };
    if !created && !process.has_permission(&metadata, permissions) {
        return Err(FsError::PermissionDenied);
    }
    let descriptor = if metadata.is_dir {
        if flags & (3 | TRUNC) != 0 {
            return Err(FsError::IsADirectory);
        }
        ProcessDescriptor::Directory(Arc::new(Mutex::new(DirectoryHandle {
            object: vfs.open_directory_object(path)?,
            path: path.into(),
            entries: vfs.read_dir(path)?,
            offset: 0,
            metadata,
        })))
    } else {
        let mut handle = vfs.open(path)?;
        if flags & TRUNC != 0 && flags & 3 != 0 {
            match handle.ops.truncate(0) {
                Ok(()) | Err(FsError::Unsupported) => {}
                Err(e) => return Err(e),
            }
        }
        ProcessDescriptor::File(Arc::new(Mutex::new(handle)))
    };
    let fd = process
        .insert_fd_with_status_flags(descriptor, flags)
        .map_err(|_| FsError::TooManyFiles)?;
    if flags & 0x80000 != 0 {
        process
            .set_fd_flags(fd, 1)
            .map_err(|_| FsError::BadDescriptor)?;
    }
    Ok(fd)
}
