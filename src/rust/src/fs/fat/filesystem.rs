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

type FatFs = fatfs::FileSystem<BufStream>;
type FatDirEntry<'a> =
    fatfs::DirEntry<'a, BufStream, fatfs::NullTimeProvider, fatfs::LossyOemCpConverter>;

pub struct Fat16FileSystem {
    fs: Arc<Mutex<FatFs>>,
}

impl Fat16FileSystem {
    pub fn new(id: usize) -> Result<Self, BlockDeviceError> {
        let fs = fatfs::FileSystem::new(BufStream::new(id), fatfs::FsOptions::new())
            .map_err(|_| BlockDeviceError::IoError)?;
        let fs = Arc::new(Mutex::new(fs));

        Ok(Self { fs })
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
            _ => root_dir.open_dir(path).map_err(fat_error)?,
        };
        let mut res = Vec::new();
        for r in parent_dir.iter() {
            let e = r.map_err(fat_error)?;
            res.push(fat_entry_name(&e));
        }
        Ok(res)
    }

    fn create(&self, path: &str, directory: bool) -> Result<(), FsError> {
        validate_create_path(path)?;
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        if directory {
            root_dir
                .create_dir(path)
                .map_err(fat_error)?;
        } else {
            root_dir
                .create_file(path)
                .map_err(fat_error)?;
        };
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), FsError> {
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        root_dir.remove(path).map_err(fat_error)
    }

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        if path.is_empty() {
            return Ok(FileMetadata {
                uid: 0,
                gid: 0,
                mode: 0o755,
                size: 0,
                is_dir: true,
            });
        }
        let fs = self.fs.lock();
        let root_dir = fs.root_dir();
        let (parent_path, entry_name) = fat_parent_and_name(path);
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
                let mode = fat_entry_mode(path, &e);
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

    fn chmod(&self, path: &str, _mode: u16) -> Result<(), FsError> {
        self.metadata(path)?;
        Err(FsError::Unsupported)
    }

    fn chown(&self, path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        self.metadata(path)?;
        Err(FsError::Unsupported)
    }
}

fn validate_create_path(path: &str) -> Result<(), FsError> {
    for component in path.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(FsError::InvalidPath);
        }
    }
    Ok(())
}

pub(super) fn fat_entry_mode(path: &str, entry: &FatDirEntry<'_>) -> u16 {
    let mut mode = if entry.is_dir() || path.to_ascii_lowercase().ends_with(".elf") {
        0o755
    } else {
        0o644
    };

    if entry.attributes().contains(fatfs::FileAttributes::READ_ONLY) {
        mode &= !0o222;
    }

    mode
}

pub(super) fn fat_entry_name(entry: &FatDirEntry<'_>) -> String {
    if let Some(name) = entry.long_file_name_as_ucs2_units() {
        return String::from_utf16_lossy(name);
    }

    String::from_utf8_lossy(entry.short_file_name_as_bytes())
        .to_ascii_lowercase()
        .to_string()
}

pub(super) fn fat_parent_and_name(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

pub(super) fn fat_error(error: fatfs::Error<BlockDeviceError>) -> FsError {
    match error {
        fatfs::Error::NotFound => FsError::NotFound,
        fatfs::Error::AlreadyExists => FsError::AlreadyExists,
        fatfs::Error::DirectoryIsNotEmpty => FsError::NotEmpty,
        fatfs::Error::NotEnoughSpace => FsError::NoSpace,
        fatfs::Error::InvalidInput
        | fatfs::Error::InvalidFileNameLength
        | fatfs::Error::UnsupportedFileNameCharacter => FsError::InvalidArgument,
        fatfs::Error::Io(_)
        | fatfs::Error::UnexpectedEof
        | fatfs::Error::WriteZero
        | fatfs::Error::CorruptedFileSystem => FsError::IoError,
        _ => FsError::IoError,
    }
}
