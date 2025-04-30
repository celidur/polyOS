use alloc::sync::Arc;

use crate::fs::{
    FsError, MountOptions,
    vfs::{FileSystem, FileSystemDriver},
};

use super::filesystem::Fat16FileSystem;

#[derive(Debug, Default)]
pub struct FatDriver;

impl FileSystemDriver for FatDriver {
    fn mount(&self, options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        let id = options.block_device_id.ok_or(FsError::InvalidArgument)?;
        let fs = Fat16FileSystem::new(id).map_err(|_| FsError::IoError)?;
        Ok(Arc::new(fs))
    }
}
