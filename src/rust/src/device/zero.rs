use alloc::boxed::Box;

use crate::fs::{FileHandle, FileMetadata, FileOps, FsError};

struct ZeroFile;

impl FileOps for ZeroFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        buf.fill(0);
        Ok(buf.len())
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

pub fn open_zero() -> FileHandle {
    FileHandle::new(Box::new(ZeroFile))
}

crate::register_device_node!(ZERO_DEVICE_NODE_REG, ["zero"], open_zero);
