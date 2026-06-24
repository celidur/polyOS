#![allow(unused)]

use core::fmt::Debug;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::RwLock;

use crate::memory::PageDirectory;

#[derive(Debug)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    InvalidPath,
    NotADirectory,
    IoError,
    Unsupported,
    PermissionDenied,
    NotEmpty,
    NoSpace,
    InvalidArgument,
    IsADirectory,
    WouldBlock,
    BrokenPipe,
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
    fn truncate(&mut self, _size: usize) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }
    fn ioctl(
        &mut self,
        _request: u32,
        _arg: u32,
        _directory: &PageDirectory,
    ) -> Result<u32, FsError> {
        Err(FsError::Unsupported)
    }
    fn stat(&self) -> Result<FileMetadata, FsError>;
}

pub struct FileHandle {
    pub ops: Box<dyn FileOps + Send + Sync>,
}

impl FileHandle {
    pub fn new(ops: Box<dyn FileOps + Send + Sync>) -> Self {
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
        mounts.push(MountEntry {
            mount_point: mount_point.to_string(),
            filesystem: fs,
        });
        Ok(())
    }

    /// Resolve `path` to candidate (FileSystem, subpath) pairs.
    ///
    /// Multiple filesystems may share the same mount point. Later mounts are
    /// searched first so a caller can intentionally layer filesystems.
    fn resolve_paths(&self, path: &str) -> Vec<(Arc<dyn FileSystem>, String)> {
        let mounts = self.mounts.read();
        let mut best_len = 0;
        let mut matches = Vec::new();
        for entry in mounts.iter() {
            let mp_len = entry.mount_point.len();
            if !mount_matches(path, &entry.mount_point) {
                continue;
            }

            if mp_len > best_len {
                best_len = mp_len;
                matches.clear();
            }

            if mp_len == best_len {
                let subpath = &path[mp_len..];
                let subpath = subpath.trim_start_matches('/');
                matches.push((entry.filesystem.clone(), subpath.to_string()));
            }
        }

        matches.reverse();
        matches
    }

    /// Resolve `path` to a single (FileSystem, subpath) pair for callers that
    /// need the top mount only.
    fn resolve_path(&self, path: &str) -> Result<(Arc<dyn FileSystem>, String), FsError> {
        self.resolve_paths(path)
            .into_iter()
            .next()
            .ok_or(FsError::NotFound)
    }

    fn existing_path(&self, path: &str) -> Result<(Arc<dyn FileSystem>, String), FsError> {
        let mut last_error = FsError::NotFound;
        for (fs, subpath) in self.resolve_paths(path) {
            match fs.metadata(subpath.as_str()) {
                Ok(_) => return Ok((fs, subpath)),
                Err(error) => last_error = error,
            }
        }

        Err(last_error)
    }

    fn create_target(&self, path: &str) -> Result<(Arc<dyn FileSystem>, String), FsError> {
        let candidates = self.resolve_paths(path);
        for (fs, subpath) in candidates.iter() {
            if subpath.is_empty() || fs.metadata(subpath.as_str()).is_ok() {
                return Err(FsError::AlreadyExists);
            }
        }

        let mut last_error = FsError::NotFound;
        for (fs, subpath) in candidates {
            match validate_create_parent(fs.as_ref(), subpath.as_str()) {
                Ok(()) => return Ok((fs, subpath)),
                Err(FsError::NotFound) => last_error = FsError::NotFound,
                Err(error) => return Err(error),
            }
        }

        Err(last_error)
    }

    pub fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let mut last_error = FsError::NotFound;
        for (fs, subpath) in self.resolve_paths(path) {
            match fs.open(&subpath) {
                Ok(handle) => return Ok(handle),
                Err(FsError::NotFound) => last_error = FsError::NotFound,
                Err(error) => return Err(error),
            }
        }

        Err(last_error)
    }

    pub fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let mut entries = Vec::new();
        let mut found = false;
        let mut last_error = FsError::NotFound;
        for (fs, subpath) in self.resolve_paths(path) {
            match fs.read_dir(&subpath) {
                Ok(fs_entries) => {
                    found = true;
                    append_unique(&mut entries, fs_entries);
                }
                Err(FsError::NotFound) => last_error = FsError::NotFound,
                Err(error) => {
                    if !found {
                        last_error = error;
                    }
                }
            }
        }

        if !found {
            return Err(last_error);
        }

        self.append_mount_points(path, &mut entries);
        Ok(entries)
    }

    pub fn create(&self, path: &str, directory: bool) -> Result<(), FsError> {
        let (fs, subpath) = self.create_target(path)?;
        fs.create(&subpath, directory)
    }

    pub fn remove(&self, path: &str) -> Result<(), FsError> {
        let (fs, subpath) = self.existing_path(path)?;
        fs.remove(&subpath)
    }

    pub fn mkdir(&self, path: &str) -> Result<(), FsError> {
        self.create(path, true)
    }

    pub fn rmdir(&self, path: &str) -> Result<(), FsError> {
        self.remove(path)
    }

    pub fn stat(&self, path: &str) -> Result<FileMetadata, FsError> {
        let (fs, subpath) = self.existing_path(path)?;
        fs.metadata(&subpath)
    }

    pub fn chmod(&self, path: &str, mode: u16) -> Result<(), FsError> {
        let (fs, subpath) = self.existing_path(path)?;
        fs.chmod(&subpath, mode)
    }

    pub fn chown(&self, path: &str, uid: u32, gid: u32) -> Result<(), FsError> {
        let (fs, subpath) = self.existing_path(path)?;
        fs.chown(&subpath, uid, gid)
    }

    fn append_mount_points(&self, path: &str, entries: &mut Vec<String>) {
        let path = normalize_mount_parent(path);
        let mounts = self.mounts.read();
        for mount in mounts.iter() {
            let Some(name) = direct_mount_child(path.as_str(), mount.mount_point.as_str()) else {
                continue;
            };

            if !entries.iter().any(|entry| entry == name.as_str()) {
                entries.push(name);
            }
        }
    }
}

fn append_unique(entries: &mut Vec<String>, new_entries: Vec<String>) {
    for entry in new_entries {
        if !entries.iter().any(|existing| existing == entry.as_str()) {
            entries.push(entry);
        }
    }
}

fn mount_matches(path: &str, mount_point: &str) -> bool {
    if mount_point == "/" {
        return path.starts_with('/');
    }

    path == mount_point
        || path
            .strip_prefix(mount_point)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn normalize_mount_parent(path: &str) -> String {
    if path.is_empty() || path == "/" {
        return "/".to_string();
    }

    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

fn direct_mount_child(parent: &str, mount_point: &str) -> Option<String> {
    if mount_point == "/" {
        return None;
    }

    let mount = mount_point.trim_end_matches('/');
    let mount = if mount.is_empty() { "/" } else { mount };

    if parent == "/" {
        let rest = mount.strip_prefix('/')?;
        if rest.is_empty() || rest.contains('/') {
            return None;
        }
        return Some(rest.to_string());
    }

    let rest = mount.strip_prefix(parent)?.strip_prefix('/')?;
    if rest.is_empty() || rest.contains('/') {
        None
    } else {
        Some(rest.to_string())
    }
}

fn validate_create_parent(fs: &dyn FileSystem, path: &str) -> Result<(), FsError> {
    if path.is_empty() {
        return Err(FsError::AlreadyExists);
    }

    let Some(parent) = parent_path(path) else {
        return Ok(());
    };

    match fs.metadata(parent) {
        Ok(metadata) if metadata.is_dir => Ok(()),
        Ok(_) => Err(FsError::NotADirectory),
        Err(error) => Err(error),
    }
}

fn parent_path(path: &str) -> Option<&str> {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .filter(|parent| !parent.is_empty())
}
