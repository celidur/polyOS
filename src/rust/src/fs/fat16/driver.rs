use alloc::sync::Arc;

use crate::{
    fs::{
        FsError, MountOptions,
        vfs::{FileSystem, FileSystemDriver},
    },
    kernel::KERNEL,
};

use super::filesystem::Fat16FileSystem;

#[derive(Debug, Default)]
pub struct Fat16Driver;

impl FileSystemDriver for Fat16Driver {
    fn mount(&self, options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        let id = options.block_device_id.ok_or(FsError::InvalidArgument)?;
        let fs = Fat16FileSystem::new(id).map_err(|_| FsError::IoError)?;
        Ok(Arc::new(fs))
    }
}
