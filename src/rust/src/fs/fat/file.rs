use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    device::bufstream::BufStream,
    fs::{
        FsError,
        vfs::{FileMetadata, FileOps},
    },
};

use fatfs::{Read, Seek, SeekFrom, Write};

use super::filesystem::{fat_entry_mode, fat_entry_name, fat_error, fat_parent_and_name};

pub struct FatFile {
    file:
        Mutex<fatfs::File<'static, BufStream, fatfs::NullTimeProvider, fatfs::LossyOemCpConverter>>,
    fs: Arc<Mutex<fatfs::FileSystem<BufStream>>>,
    path: Arc<str>,
    offset: usize,
}

impl FatFile {
    pub fn new(fs: Arc<Mutex<fatfs::FileSystem<BufStream>>>, path: &str) -> Result<Self, FsError> {
        let file: fatfs::File<'static, _, _, _> = {
            let fs = fs.lock();
            let root_dir = fs.root_dir();
            // SAFETY HACK: Extend lifetime
            unsafe { core::mem::transmute(root_dir.open_file(path).map_err(fat_error)?) }
        };

        Ok(FatFile {
            file: Mutex::new(file),
            fs,
            path: Arc::from(path),
            offset: 0,
        })
    }
}

impl Drop for FatFile {
    fn drop(&mut self) {
        let _ = self.file.lock().flush();
    }
}

unsafe impl Send for FatFile {}
unsafe impl Sync for FatFile {}

impl FileOps for FatFile {
    fn position(&self) -> Result<usize, FsError> {
        Ok(self.offset)
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        let mut file = self.file.lock();
        let position = file
            .seek(SeekFrom::Start(self.offset as u64))
            .map_err(fat_error)?;
        if position < self.offset as u64 {
            return Ok(0);
        }
        let count = file.read(buf).map_err(fat_error)?;
        self.offset += count;
        Ok(count)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut file = self.file.lock();
        let mut position = file
            .seek(SeekFrom::Start(self.offset as u64))
            .map_err(fat_error)? as usize;
        let zeros = [0u8; 512];
        while position < self.offset {
            let n = (self.offset - position).min(zeros.len());
            file.write_all(&zeros[..n]).map_err(fat_error)?;
            position += n;
        }
        let count = file.write(buf).map_err(fat_error)?;
        self.offset += count;
        file.flush().map_err(fat_error)?;
        Ok(count)
    }

    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        self.offset = pos;
        Ok(pos)
    }

    fn truncate(&mut self, size: usize) -> Result<(), FsError> {
        let mut file = self.file.lock();
        let mut position = file.seek(SeekFrom::Start(size as u64)).map_err(fat_error)? as usize;
        let zeros = [0u8; 512];
        while position < size {
            let n = (size - position).min(zeros.len());
            file.write_all(&zeros[..n]).map_err(fat_error)?;
            position += n;
        }
        file.truncate().map_err(fat_error)?;
        file.flush().map_err(fat_error)
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        let (parent_path, entry_name) = fat_parent_and_name(self.path.as_ref());
        let parent_dir = if parent_path.is_empty() {
            Ok(root_dir)
        } else {
            root_dir.open_dir(parent_path)
        };
        let parent_dir = parent_dir.map_err(fat_error)?;
        let entry_name = entry_name.to_lowercase();
        for r in parent_dir.iter() {
            let e = r.map_err(fat_error)?;
            let name = fat_entry_name(&e).to_lowercase();

            if name == entry_name {
                let size = e.len();
                let mode = fat_entry_mode(self.path.as_ref(), &e);
                return Ok(FileMetadata {
                    uid: 0,
                    gid: 0,
                    mode,
                    size,
                    is_dir: e.is_dir(),
                });
            }
        }
        Err(FsError::NotFound)
    }
}
