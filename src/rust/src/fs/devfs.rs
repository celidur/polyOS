use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use crate::device::{device_node_names, find_device_node};

use super::vfs::{FileHandle, FileMetadata, FileSystem, FileSystemDriver, FsError, MountOptions};

#[derive(Debug, Default)]
pub struct DevFsDriver;

impl FileSystemDriver for DevFsDriver {
    fn mount(&self, _options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        Ok(Arc::new(DevFs))
    }
}

struct DevFs;

impl FileSystem for DevFs {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let name = normalize(path);
        let node = find_device_node(name.as_str()).ok_or(FsError::NotFound)?;
        Ok((node.open)())
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        if !normalize(path).is_empty() {
            return Err(FsError::NotFound);
        }

        Ok(device_node_names())
    }

    fn create(&self, _path: &str, _directory: bool) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn remove(&self, _path: &str) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        if normalize(path).is_empty() {
            return Ok(metadata(0o755, true));
        }

        self.open(path)?;
        Ok(metadata(0o666, false))
    }

    fn chmod(&self, _path: &str, _mode: u16) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn chown(&self, _path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }
}

fn normalize(path: &str) -> String {
    path.trim_matches('/').to_string()
}

fn metadata(mode: u16, is_dir: bool) -> FileMetadata {
    FileMetadata {
        uid: 0,
        gid: 0,
        mode,
        size: 0,
        is_dir,
    }
}
