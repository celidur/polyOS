use alloc::sync::Arc;

use crate::{
    device::block_dev::get_block_device,
    fs::{
        vfs::{FileSystem, FileSystemDriver},
        FsError, MountOptions,
    },
};

use super::filesystem::Fat16FileSystem;

pub struct Fat16Driver;

impl FileSystemDriver for Fat16Driver {
    fn mount(&self, options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        let id = options.block_device_id.ok_or(FsError::InvalidArgument)?;
        let dev = get_block_device(id).ok_or(FsError::NotFound)?;
        let fs = Fat16FileSystem::new(dev).map_err(|_| FsError::IoError)?;
        Ok(Arc::new(fs))
    }
}
