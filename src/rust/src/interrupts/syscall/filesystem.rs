//! Filesystem ABI marshalling; path policy and open-object operations live in fs.
use super::{abi, file, user};
use crate::{
    constant::MAX_PATH,
    fs::{FileMetadata, FsError, descriptor::ProcessDescriptor, path},
    kernel::KERNEL,
    schedule::process::Process,
};
use alloc::sync::Arc;
fn process() -> Option<Arc<Process>> {
    KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
}
fn pathname(process: &Process, fd: i32, ptr: u32) -> Result<alloc::string::String, FsError> {
    let name = user::read_process_string(process, ptr, MAX_PATH).ok_or(FsError::BadAddress)?;
    let path = path::resolve_at(process, fd, &name)?;
    crate::fs::access::search(process, &path)?;
    Ok(path)
}
pub fn syscall_openat(fd: u32, ptr: u32, flags: u32, mode: u32) -> u32 {
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let path = match pathname(&p, fd as i32, ptr) {
        Ok(p) => p,
        Err(e) => return file::fs_errno(e),
    };
    match crate::fs::open::open(&p, &path, flags, mode as u16) {
        Ok(fd) => fd as u32,
        Err(e) => file::fs_errno(e),
    }
}
pub fn syscall_access(ptr: u32, mode: u32) -> u32 {
    if mode & !7 != 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let path = match pathname(&p, path::AT_FDCWD, ptr) {
        Ok(p) => p,
        Err(e) => return file::fs_errno(e),
    };
    let path = match p.follow_path(&path) {
        Ok(path) => path,
        Err(e) => return file::fs_errno(e),
    };
    let meta = match KERNEL.vfs.read().stat(&path) {
        Ok(m) => m,
        Err(e) => return file::fs_errno(e),
    };
    if p.control
        .lock()
        .credentials
        .permission(&meta, mode as u16, true)
    {
        0
    } else {
        abi::errno(abi::EACCES)
    }
}
pub fn syscall_ftruncate(fd: u32, length: u32) -> u32 {
    if (length as i32) < 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    if p.get_status_flags(fd as i32)
        .map(|f| f & 3 == 0)
        .unwrap_or(true)
    {
        return abi::errno(abi::EBADF);
    }
    match p.get_fd(fd as i32) {
        Some(ProcessDescriptor::File(f)) => match f.lock().ops.truncate(length as usize) {
            Ok(()) => 0,
            Err(e) => file::fs_errno(e),
        },
        Some(_) => abi::errno(abi::EINVAL),
        None => abi::errno(abi::EBADF),
    }
}
pub fn syscall_fchmod(fd: u32, mode: u32) -> u32 {
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let Some(d) = p.get_fd(fd as i32) else {
        return abi::errno(abi::EBADF);
    };
    let meta = match d.stat() {
        Ok(m) => m,
        Err(e) => return file::fs_errno(e),
    };
    if p.euid() != 0 && p.euid() != meta.uid {
        return abi::errno(abi::EPERM);
    }
    let result = match d {
        ProcessDescriptor::File(f) => f.lock().ops.chmod(mode as u16 & 0o777),
        ProcessDescriptor::Directory(d) => {
            let mut d = d.lock();
            match &mut d.object {
                Some(object) => object.ops.chmod(mode as u16 & 0o777),
                None => Err(FsError::Unsupported),
            }
        }
        _ => Err(FsError::Unsupported),
    };
    match result {
        Ok(()) => 0,
        Err(e) => file::fs_errno(e),
    }
}
pub fn syscall_readlink(ptr: u32, buf: u32, len: u32) -> u32 {
    if len == 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let path = match pathname(&p, path::AT_FDCWD, ptr) {
        Ok(p) => p,
        Err(e) => return file::fs_errno(e),
    };
    let target = match KERNEL.vfs.read().read_link(&path) {
        Ok(t) => t,
        Err(e) => return file::fs_errno(e),
    };
    let count = target.len().min(len as usize);
    if user::copy_to_user(&p.page_directory, buf, target.as_ptr(), count as u32).is_err() {
        return abi::errno(abi::EFAULT);
    }
    count as u32
}
/// Linux i386 stat64 layout (96 bytes); legacy PolyOS stat calls remain separate.
#[repr(C, packed)]
#[derive(Default)]
struct Stat64 {
    dev: u64,
    pad0: u32,
    legacy_ino: u32,
    mode: u32,
    nlink: u32,
    uid: u32,
    gid: u32,
    rdev: u64,
    pad1: u32,
    size: i64,
    blksize: u32,
    blocks: u64,
    atime: u32,
    atime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    ctime: u32,
    ctime_nsec: u32,
    ino: u64,
}
const _: [(); 96] = [(); core::mem::size_of::<Stat64>()];
fn write_stat(p: &Process, ptr: u32, meta: FileMetadata) -> u32 {
    let stat = Stat64 {
        mode: file::stat_mode(&meta),
        nlink: 1,
        uid: meta.uid,
        gid: meta.gid,
        size: meta.size as i64,
        blksize: 512,
        blocks: (meta.size + 511) / 512,
        ..Default::default()
    };
    if user::write_value(&p.page_directory, ptr, &stat).is_err() {
        abi::errno(abi::EFAULT)
    } else {
        0
    }
}
fn metadata_at(p: &Process, fd: i32, ptr: u32, flags: u32) -> Result<FileMetadata, FsError> {
    let name = user::read_process_string(p, ptr, MAX_PATH).ok_or(FsError::BadAddress)?;
    if name.is_empty() && flags & 0x1000 != 0 {
        return if fd == path::AT_FDCWD {
            KERNEL.vfs.read().stat(&p.cwd.lock())
        } else {
            p.get_fd(fd).ok_or(FsError::BadDescriptor)?.stat()
        };
    }
    let path = path::resolve_at(p, fd, &name)?;
    crate::fs::access::search(p, &path)?;
    if flags & 0x100 != 0 {
        KERNEL.vfs.read().lstat(&path)
    } else {
        KERNEL.vfs.read().stat(&p.follow_path(&path)?)
    }
}
pub fn syscall_fstatat(fd: u32, path: u32, ptr: u32, flags: u32) -> u32 {
    if flags & !(0x100 | 0x1000) != 0 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    match metadata_at(&p, fd as i32, path, flags) {
        Ok(m) => write_stat(&p, ptr, m),
        Err(e) => file::fs_errno(e),
    }
}
pub fn syscall_stat64(path: u32, ptr: u32) -> u32 {
    syscall_fstatat((-100i32) as u32, path, ptr, 0)
}
pub fn syscall_lstat64(path: u32, ptr: u32) -> u32 {
    syscall_fstatat((-100i32) as u32, path, ptr, 0x100)
}
pub fn syscall_fstat64(fd: u32, ptr: u32) -> u32 {
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    match p
        .get_fd(fd as i32)
        .ok_or(FsError::BadDescriptor)
        .and_then(|d| d.stat())
    {
        Ok(m) => write_stat(&p, ptr, m),
        Err(e) => file::fs_errno(e),
    }
}
pub(super) fn legacy_lstat(path: u32, ptr: u32) -> u32 {
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    match metadata_at(&p, path::AT_FDCWD, path, 0x100) {
        Ok(m) => {
            if user::write_value(&p.page_directory, ptr, &file::metadata_to_stat(&m)).is_ok() {
                0
            } else {
                abi::errno(abi::EFAULT)
            }
        }
        Err(e) => file::fs_errno(e),
    }
}
pub fn syscall_getdents64(fd: u32, ptr: u32, len: u32) -> u32 {
    let Some(p) = process() else {
        return abi::errno(abi::ESRCH);
    };
    let d = match p.get_fd(fd as i32) {
        Some(ProcessDescriptor::Directory(d)) => d,
        Some(_) => return abi::errno(abi::ENOTDIR),
        None => return abi::errno(abi::EBADF),
    };
    let len = len.min(65536) as usize;
    if user::validate_range(&p, ptr, len as u32, true).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let mut d = d.lock();
    let mut written = 0;
    while d.offset < d.entries.len() {
        let name = &d.entries[d.offset];
        let record_len = (19 + name.len() + 1 + 7) & !7;
        if record_len > len - written {
            return if written == 0 {
                abi::errno(abi::EINVAL)
            } else {
                written as u32
            };
        }
        let mut record = vec![0u8; record_len];
        let index = d.offset as u64 + 1;
        record[0..8].copy_from_slice(&index.to_ne_bytes());
        record[8..16].copy_from_slice(&index.to_ne_bytes());
        record[16..18].copy_from_slice(&(record_len as u16).to_ne_bytes());
        let path = alloc::format!("{}/{}", d.path.trim_end_matches('/'), name);
        record[18] = match KERNEL.vfs.read().lstat(&path) {
            Ok(m) if m.mode & 0o170000 == 0o120000 => 10,
            Ok(m) if m.is_dir => 4,
            Ok(_) => 8,
            Err(_) => 0,
        };
        record[19..19 + name.len()].copy_from_slice(name.as_bytes());
        if user::copy_to_user(
            &p.page_directory,
            ptr + written as u32,
            record.as_ptr(),
            record_len as u32,
        )
        .is_err()
        {
            return abi::errno(abi::EFAULT);
        }
        d.offset += 1;
        written += record_len;
    }
    written as u32
}
