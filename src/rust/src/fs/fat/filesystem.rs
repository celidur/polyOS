use crate::{
    device::{block_dev::BlockDeviceError, bufstream::BufStream},
    fs::{
        FsError,
        vfs::{FileHandle, FileMetadata, FileSystem},
    },
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::mutex::Mutex;

use super::file::FatFile;

pub struct Fat16FileSystem {
    fs: Arc<Mutex<fatfs::FileSystem<BufStream>>>,
}

impl Fat16FileSystem {
    pub fn new(id: usize) -> Result<Self, BlockDeviceError> {
        let fs = fatfs::FileSystem::new(BufStream::new(id), fatfs::FsOptions::new())
            .map_err(|_| BlockDeviceError::IoError)?;

        Ok(Self {
            fs: Arc::new(Mutex::new(fs)),
        })
    }
}

impl FileSystem for Fat16FileSystem {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        Ok(FileHandle::new(Box::new(FatFile::new(
            self.fs.clone(),
            path,
        )?)))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        let parent_dir = match path {
            "." => root_dir,
            "" => root_dir,
            _ => root_dir.open_dir(path).map_err(|_| FsError::NotFound)?,
        };
        let mut res = Vec::new();
        for r in parent_dir.iter() {
            let e = r.map_err(|_| FsError::NotFound)?;
            let short = e.short_file_name_as_bytes();
            let name = if let Some(name) = e.long_file_name_as_ucs2_units() {
                String::from_utf16_lossy(name)
            } else {
                String::from_utf8_lossy(short).to_string()
            };

            res.push(name);
        }
        Ok(res)
    }

    fn create(&self, path: &str, directory: bool) -> Result<(), FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        if directory {
            root_dir
                .create_dir(path)
                .map_err(|_| FsError::AlreadyExists)?;
        } else {
            root_dir
                .create_file(path)
                .map_err(|_| FsError::AlreadyExists)?;
        };
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        if root_dir.remove(path).is_err() {
            return Err(FsError::NotFound);
        }
        Ok(())
    }

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        let parent_dir = path.rsplit('/').nth(1).unwrap_or("");
        let parent_dir = if parent_dir.is_empty() {
            Ok(root_dir)
        } else {
            root_dir.open_dir(parent_dir)
        };
        if parent_dir.is_err() {
            return Err(FsError::NotFound);
        }
        let n = path.rsplit('/').next().unwrap_or("");
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

    fn chmod(&self, _path: &str, _mode: u16) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn chown(&self, _path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }
}
