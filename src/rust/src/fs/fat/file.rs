use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    device::bufstream::BufStream,
    fs::{FsError, vfs::FileOps},
};

use fatfs::{Read, Seek, SeekFrom, Write};

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
            unsafe {
                core::mem::transmute(root_dir.open_file(path).map_err(|_| FsError::NotFound)?)
            }
        };

        Ok(FatFile {
            file: Mutex::new(file),
            fs,
            path: Arc::from(path),
        })
    }
}

unsafe impl Send for FatFile {}

impl FileOps for FatFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        self.file.lock().read(buf).map_err(|_| FsError::IoError)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        let res = self.file.lock().write(buf).map_err(|_| FsError::IoError);

        drop(self.file.lock());

        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        // SAFETY HACK: Extend lifetime
        let file: fatfs::File<'static, _, _, _> = unsafe {
            core::mem::transmute(
                root_dir
                    .open_file(&self.path)
                    .map_err(|_| FsError::NotFound)?,
            )
        };

        self.file = Mutex::new(file);
        res
    }

    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        self.file
            .lock()
            .seek(SeekFrom::Start(pos as u64))
            .map_err(|_| FsError::IoError)
            .map(|d| d as usize)
    }
}
