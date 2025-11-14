use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use spin::Mutex;

use crate::{
    device::bufstream::BufStream,
    fs::{
        FsError,
        vfs::{FileMetadata, FileOps},
    },
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
unsafe impl Sync for FatFile {}

impl FileOps for FatFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        self.file.lock().read(buf).map_err(|_| FsError::IoError)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        let res = self.file.lock().write(buf).map_err(|_| FsError::IoError);
        self.file.lock().truncate().map_err(|_| FsError::IoError)?;
        self.file.lock().flush().map_err(|_| FsError::IoError)?;

        res
    }

    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        self.file
            .lock()
            .seek(SeekFrom::Start(pos as u64))
            .map_err(|_| FsError::IoError)
            .map(|d| d as usize)
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        let parent_dir = self.path.rsplit('/').nth(1).unwrap_or("");
        let parent_dir = if parent_dir.is_empty() {
            Ok(root_dir)
        } else {
            root_dir.open_dir(parent_dir)
        };
        if parent_dir.is_err() {
            return Err(FsError::NotFound);
        }
        let n = self.path.rsplit('/').next().unwrap_or("");
        let parent_dir = parent_dir.unwrap();
        for r in parent_dir.iter() {
            let e = r.map_err(|_| FsError::NotFound)?;
            let short = e.short_file_name_as_bytes();
            let name = if let Some(name) = e.long_file_name_as_ucs2_units() {
                String::from_utf16_lossy(name)
            } else {
                String::from_utf8_lossy(short).to_string()
            };
            let name = name.to_lowercase();
            let n = n.to_lowercase();

            if name == n {
                let size = e.len();
                let mode = if e.is_dir() { 0o755 } else { 0o644 };
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
