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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        self.file.lock().read(buf).map_err(fat_error)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        self.file.lock().write(buf).map_err(fat_error)
    }

    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        self.file
            .lock()
            .seek(SeekFrom::Start(pos as u64))
            .map_err(fat_error)
            .map(|d| d as usize)
    }

    fn truncate(&mut self, size: usize) -> Result<(), FsError> {
        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(size as u64))
            .map_err(fat_error)?;
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
