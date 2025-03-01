use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::{Mutex, RwLock};

use super::vfs::{
    FileHandle, FileMetadata, FileOps, FileSystem, FileSystemDriver, FsError, MountOptions,
};

pub struct MemFsDriver;

impl FileSystemDriver for MemFsDriver {
    fn mount(&self, _options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        Ok(Arc::new(MemFsVolume::new()))
    }
}

#[derive(Clone)]
struct MemNode {
    is_dir: bool,
    data: Vec<u8>,
    meta: FileMetadata,
}

pub struct MemFsVolume {
    inner: RwLock<BTreeMap<String, Arc<Mutex<MemNode>>>>,
}

impl MemFsVolume {
    pub fn new() -> Self {
        let mut map = BTreeMap::new();
        // root node
        let root = MemNode {
            is_dir: true,
            data: Vec::new(),
            meta: FileMetadata {
                uid: 0,
                gid: 0,
                mode: 0o755,
                size: 0,
                is_dir: true,
            },
        };
        map.insert("".to_string(), Arc::new(Mutex::new(root)));
        Self {
            inner: RwLock::new(map),
        }
    }
}

impl FileSystem for MemFsVolume {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let map = self.inner.read();
        let node = map.get(path).ok_or(FsError::NotFound)?;
        if node.lock().is_dir {
            return Err(FsError::IoError); // can't open dir as file
        }
        let handle = MemFsFile {
            inner: node.clone(),
            offset: 0,
        };
        Ok(FileHandle::new(Box::new(handle)))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let map = self.inner.read();
        let node = map.get(path).ok_or(FsError::NotFound)?;
        if !node.lock().is_dir {
            return Err(FsError::IoError);
        }
        let mut result = Vec::new();
        // gather direct children
        let prefix = if path.is_empty() {
            ""
        } else {
            path.trim_end_matches('/')
        };
        for (k, v) in map.iter() {
            if k == path {
                continue;
            }
            if v.lock().is_dir {
                // check if 'k' starts with prefix, etc. Simplify for demo
            }
            if k.starts_with(prefix) && k != path {
                result.push(k.clone());
            }
        }
        Ok(result)
    }

    fn create(&self, path: &str, directory: bool) -> Result<(), FsError> {
        let mut map = self.inner.write();
        if map.contains_key(path) {
            return Err(FsError::AlreadyExists);
        }
        let new_meta = FileMetadata {
            uid: 0,
            gid: 0,
            mode: if directory { 0o755 } else { 0o644 },
            size: 0,
            is_dir: directory,
        };
        let new_node = MemNode {
            is_dir: directory,
            data: Vec::new(),
            meta: new_meta,
        };
        map.insert(path.to_string(), Arc::new(Mutex::new(new_node)));
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), FsError> {
        let mut map = self.inner.write();
        let node = map.remove(path).ok_or(FsError::NotFound)?;
        let node = node.lock();
        if node.is_dir && !node.data.is_empty() {
            return Err(FsError::NotEmpty);
        }
        Ok(())
    }

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        let map = self.inner.read();
        let node = map.get(path).ok_or(FsError::NotFound)?.lock();
        Ok(node.meta.clone())
    }

    fn chmod(&self, path: &str, mode: u16) -> Result<(), FsError> {
        let mut map = self.inner.write();
        let mut node = map.get_mut(path).ok_or(FsError::NotFound)?.lock();
        node.meta.mode = mode;
        Ok(())
    }

    fn chown(&self, path: &str, uid: u32, gid: u32) -> Result<(), FsError> {
        let mut map = self.inner.write();
        let mut node = map.get_mut(path).ok_or(FsError::NotFound)?.lock();
        node.meta.uid = uid;
        node.meta.gid = gid;
        Ok(())
    }
}

struct MemFsFile {
    inner: Arc<Mutex<MemNode>>,
    offset: usize,
}

impl FileOps for MemFsFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        let node = self.inner.lock();
        let available = node.data.len().saturating_sub(self.offset);
        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&node.data[self.offset..self.offset + to_read]);
        self.offset += to_read;
        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        let mut node = self.inner.lock();
        let end = self.offset + buf.len();
        if end > node.data.len() {
            node.data.resize(end, 0);
        }
        node.data[self.offset..end].copy_from_slice(buf);
        self.offset += buf.len();
        node.meta.size = node.data.len() as u64;
        Ok(buf.len())
    }

    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        let node = self.inner.lock();
        self.offset = pos.min(node.data.len());
        Ok(self.offset)
    }
}
