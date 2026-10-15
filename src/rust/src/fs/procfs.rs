use super::{FileHandle, FileMetadata, FileSystem, FileSystemDriver, FsError, MountOptions};
use crate::kernel::KERNEL;
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
#[derive(Debug)]
pub struct ProcFsDriver;
impl FileSystemDriver for ProcFsDriver {
    fn mount(&self, _options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        Ok(Arc::new(ProcFs))
    }
}
struct ProcFs;
fn executable() -> Result<String, FsError> {
    KERNEL
        .with_task_manager(|tm| {
            tm.get_current()
                .map(|t| t.read().process.executable_path.clone())
        })
        .ok_or(FsError::NotFound)
}
impl FileSystem for ProcFs {
    fn open(&self, _path: &str) -> Result<FileHandle, FsError> {
        Err(FsError::IsADirectory)
    }
    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        match path.trim_matches('/') {
            "" => Ok(vec!["self".to_string()]),
            "self" => Ok(vec!["exe".to_string()]),
            _ => Err(FsError::NotADirectory),
        }
    }
    fn read_link(&self, path: &str) -> Result<String, FsError> {
        if path.trim_matches('/') == "self/exe" {
            executable()
        } else {
            Err(FsError::InvalidArgument)
        }
    }
    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        let (mode, size, is_dir) = match path.trim_matches('/') {
            "" | "self" => (0o555, 0, true),
            "self/exe" => (0o120777, executable()?.len() as u64, false),
            _ => return Err(FsError::NotFound),
        };
        Ok(FileMetadata {
            uid: 0,
            gid: 0,
            mode,
            size,
            is_dir,
        })
    }
    fn create(&self, _path: &str, _directory: bool) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn remove(&self, _path: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn chmod(&self, _path: &str, _mode: u16) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn chown(&self, _path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
}
