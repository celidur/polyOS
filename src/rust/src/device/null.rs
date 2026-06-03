use alloc::boxed::Box;

use crate::fs::{FileHandle, FileMetadata, FileOps, FsError};

struct NullFile;

impl FileOps for NullFile {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, FsError> {
        Ok(0)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        Ok(buf.len())
    }

    fn seek(&mut self, _pos: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(metadata(0o666, false))
    }
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

pub fn open_null() -> FileHandle {
    FileHandle::new(Box::new(NullFile))
}

crate::register_device_node!(NULL_DEVICE_NODE_REG, ["null"], open_null);
