#![allow(unused)]

use core::{default, fmt::Debug};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::RwLock;

#[derive(Debug)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    InvalidPath,
    IoError,
    Unsupported,
    PermissionDenied,
    NotEmpty,
    NoSpace,
    InvalidArgument,
    IsADirectory,
}

#[derive(Debug, Clone, Default)]
pub struct FileMetadata {
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
    pub size: u64,
    pub is_dir: bool,
    // etc. e.g. timestamps if you want
}

#[derive(Clone, Default)]
pub struct MountOptions {
    pub fs_name: String,
    pub block_device_id: Option<usize>,
}

pub trait FileSystemDriver: Send + Sync + Debug {
    /// Create a new FileSystem instance given the mount options.
    /// E.g. parse the block device if needed, load superblock, etc.
    fn mount(&self, options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError>;
}

pub trait FileSystem: Send + Sync {
    fn open(&self, path: &str) -> Result<FileHandle, FsError>;
    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError>;
    fn create(&self, path: &str, directory: bool) -> Result<(), FsError>;
    fn remove(&self, path: &str) -> Result<(), FsError>;

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError>;
    fn chmod(&self, path: &str, mode: u16) -> Result<(), FsError>;
    fn chown(&self, path: &str, uid: u32, gid: u32) -> Result<(), FsError>;
}

pub trait FileOps {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError>;
    fn seek(&mut self, pos: usize) -> Result<usize, FsError>;
    fn stat(&self) -> Result<FileMetadata, FsError>;
}

pub struct FileHandle {
    pub ops: Box<dyn FileOps + Send>,
}

impl FileHandle {
    pub fn new(ops: Box<dyn FileOps + Send>) -> Self {
        Self { ops }
    }
}

struct MountEntry {
    mount_point: String,
    filesystem: Arc<dyn FileSystem>,
}

#[derive(Default)]
pub struct Vfs {
    drivers: BTreeMap<String, Arc<dyn FileSystemDriver>>,
    mounts: RwLock<Vec<MountEntry>>,
}

impl Vfs {
    pub fn new() -> Self {
        Self {
            drivers: BTreeMap::new(),
            mounts: RwLock::new(Vec::new()),
        }
    }

    /// Register a driver under a name. E.g. "fat16" -> `Fat16Driver`.
    pub fn register_fs_driver(&mut self, name: &str, driver: Arc<dyn FileSystemDriver>) {
        self.drivers.insert(name.to_string(), driver);
    }

    /// Mount the given filesystem at `mount_point`, using the provided options.
    pub fn mount(&self, mount_point: &str, options: &MountOptions) -> Result<(), FsError> {
        let driver = self
            .drivers
            .get(&options.fs_name)
            .ok_or(FsError::NotFound)?;
        let fs = driver.mount(options)?;

        let mut mounts = self.mounts.write();
        if mounts.iter().any(|m| m.mount_point == mount_point) {
            return Err(FsError::AlreadyExists);
        }
        mounts.push(MountEntry {
            mount_point: mount_point.to_string(),
            filesystem: fs,
        });
        Ok(())
    }

    /// Resolve `path` to a (FileSystem, subpath) pair by scanning mounts.
    fn resolve_path(&self, path: &str) -> Result<(Arc<dyn FileSystem>, String), FsError> {
        let mounts = self.mounts.read();
        let mut best_match: Option<(Arc<dyn FileSystem>, usize)> = None;
        for entry in mounts.iter() {
            let mp_len = entry.mount_point.len();
            if path.starts_with(&entry.mount_point)
                && (best_match.is_none() || mp_len > best_match.as_ref().unwrap().1)
            {
                best_match = Some((entry.filesystem.clone(), mp_len));
            }
        }
        if let Some((fs, mp_len)) = best_match {
            let subpath = &path[mp_len..];
            let subpath = subpath.trim_start_matches('/');
            Ok((fs, subpath.to_string()))
        } else {
            Err(FsError::NotFound)
        }
    }

    pub fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.open(&subpath)
    }

    pub fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.read_dir(&subpath)
    }

    pub fn create(&self, path: &str, directory: bool) -> Result<(), FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.create(&subpath, directory)
    }

    pub fn remove(&self, path: &str) -> Result<(), FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.remove(&subpath)
    }

    pub fn mkdir(&self, path: &str) -> Result<(), FsError> {
        self.create(path, true)
    }

    pub fn rmdir(&self, path: &str) -> Result<(), FsError> {
        self.remove(path)
    }

    pub fn stat(&self, path: &str) -> Result<FileMetadata, FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.metadata(&subpath)
    }

    pub fn chmod(&self, path: &str, mode: u16) -> Result<(), FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.chmod(&subpath, mode)
    }

    pub fn chown(&self, path: &str, uid: u32, gid: u32) -> Result<(), FsError> {
        let (fs, subpath) = self.resolve_path(path)?;
        fs.chown(&subpath, uid, gid)
    }
}
