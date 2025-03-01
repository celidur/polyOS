use crate::device::block_dev::BlockDeviceError;
use crate::fs::vfs::FileOps;
use crate::fs::FsError;
use alloc::sync::Arc;

use super::filesystem::Fat16FileSystem;

pub struct FatFile {
    fs: Arc<Fat16FileSystem>,
    start_cluster: u16,
    offset: u32,
    size: u32,
    dir_entry_loc: (u16, usize),
}

impl FatFile {
    pub fn new(
        fs: Arc<Fat16FileSystem>,
        start_cluster: u16,
        size: u32,
        dir_entry_loc: (u16, usize),
    ) -> Self {
        Self {
            fs,
            start_cluster,
            offset: 0,
            size,
            dir_entry_loc,
        }
    }
}

impl FileOps for FatFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        if self.offset as usize >= self.size as usize {
            return Ok(0);
        }
        let to_read = core::cmp::min(buf.len(), (self.size - self.offset) as usize);
        let n = self
            .fs
            .read_file_data(
                self.start_cluster,
                self.offset as usize,
                &mut buf[..to_read],
                self.size,
            )
            .map_err(|_| FsError::IoError)?;
        self.offset += n as u32;
        Ok(n)
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        let written = self
            .fs
            .write_file_data(&mut self.start_cluster, &mut self.size, self.offset, buf)
            .map_err(|e| match e {
                BlockDeviceError::NoSpace => FsError::NoSpace,
                _ => FsError::IoError,
            })?;
        self.offset += written as u32;
        self.fs
            .update_dir_entry_size(self.dir_entry_loc, self.start_cluster, self.size)?;
        Ok(written)
    }
    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        self.offset = pos as u32;
        Ok(pos)
    }
}
