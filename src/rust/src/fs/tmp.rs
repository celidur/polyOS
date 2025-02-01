use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use spin::Mutex;

use super::{path::Path, File, FileMode, FileSystem, FsError};

#[derive(Debug)]
pub enum Inode {
    File(Vec<u8>),
    Directory(BTreeMap<String, Arc<Mutex<Inode>>>),
}

pub struct TmpFile {
    path: Path,
    mode: FileMode,
    offset: u64,
    inode: Arc<Mutex<Inode>>,
}

impl File for TmpFile {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FsError> {
        let mut inode = self.inode.lock();
        match &mut *inode {
            Inode::File(data) => {
                let mut bytes_read = 0;
                for (i, byte) in data.iter().skip(self.offset as usize).enumerate() {
                    if i >= buffer.len() {
                        break;
                    }
                    buffer[i] = *byte;
                    bytes_read += 1;
                }
                self.offset += bytes_read as u64;
                Ok(bytes_read)
            }
            Inode::Directory(_) => Err(FsError::PermissionDenied),
        }
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, FsError> {
        if self.mode == FileMode::Read {
            return Err(FsError::PermissionDenied);
        }
        let mut inode = self.inode.lock();
        match &mut *inode {
            Inode::File(data) => {
                let mut bytes_written = 0;
                for (i, byte) in buffer.iter().enumerate() {
                    if i >= data.len() {
                        data.push(*byte);
                    } else {
                        data[i] = *byte;
                    }
                    bytes_written += 1;
                }
                self.offset += bytes_written as u64;
                Ok(bytes_written)
            }
            Inode::Directory(_) => Err(FsError::PermissionDenied),
        }
    }

    fn seek(&mut self, offset: u64, mode: super::SeekMode) -> Result<(), FsError> {
        match mode {
            super::SeekMode::Set => self.offset = offset,
            super::SeekMode::Cur => self.offset += offset,
            super::SeekMode::End => self.offset = self.get_size() + offset,
        }
        Ok(())
    }

    fn stat(&self) -> Result<super::FileStat, FsError> {
        let inode = self.inode.lock();
        match &*inode {
            Inode::File(data) => Ok(super::FileStat {
                size: data.len() as u32,
                flags: super::FileStatFlags::IS_FILE,
            }),
            Inode::Directory(_) => Err(FsError::PermissionDenied),
        }
    }

    fn get_position(&self) -> u64 {
        self.offset
    }

    fn get_size(&self) -> u64 {
        let inode = self.inode.lock();
        match &*inode {
            Inode::File(data) => data.len() as u64,
            Inode::Directory(_) => 0,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug)]
pub struct TmpFileSystem {
    mount_point: Path,
    inodes: Arc<Mutex<Inode>>,
}

impl TmpFileSystem {
    pub fn new(mount_point: &str) -> Self {
        let mount_point = Path::new(mount_point);
        let inodes = Arc::new(Mutex::new(Inode::Directory(BTreeMap::new())));
        Self {
            mount_point,
            inodes,
        }
    }

    fn get_directory(&self, path: &Path) -> Option<Arc<Mutex<Inode>>> {
        let mut directory: Arc<Mutex<Inode>> = self.inodes.clone();
        for p in path.components.iter() {
            let d = match &*directory.lock() {
                Inode::File(_) => None,
                Inode::Directory(children) => children.get(p).cloned(),
            };
            match d {
                Some(inode) => {
                    directory = inode;
                }
                None => {
                    return None;
                }
            }
        }
        Some(directory)
    }

    fn get_directory_create(&mut self, path: &Path) -> Option<Arc<Mutex<Inode>>> {
        let mut directory: Arc<Mutex<Inode>> = self.inodes.clone();
        for p in path.components.iter().take(path.components.len() - 1) {
            let d = match &*directory.lock() {
                Inode::File(_) => None,
                Inode::Directory(children) => children.get(p).cloned(),
            };
            match d {
                Some(inode) => {
                    directory = inode;
                }
                None => {
                    let new_inode = Arc::new(Mutex::new(Inode::Directory(BTreeMap::new())));
                    match &mut *directory.lock() {
                        Inode::File(_) => return None,
                        Inode::Directory(children) => {
                            children.insert(p.clone(), new_inode.clone());
                        }
                    }
                    directory = new_inode;
                }
            }
        }
        Some(directory)
    }
}

impl FileSystem for TmpFileSystem {
    fn open(&mut self, path: &str, mode: FileMode) -> Result<Box<dyn File>, FsError> {
        let path = Path::new(path);
        let child_path = path
            .relative_to(&self.mount_point.as_string())
            .ok_or(FsError::NotFound)?;

        let directory = if mode == FileMode::Write {
            self.get_directory_create(&child_path)
                .ok_or(FsError::PermissionDenied)?
        } else {
            self.get_directory(&child_path).ok_or(FsError::NotFound)?
        };

        let inode = match &mut *directory.lock() {
            Inode::File(_) => return Err(FsError::PermissionDenied),
            Inode::Directory(children) => {
                match children.get(child_path.components.last().unwrap()) {
                    Some(inode) => inode.clone(),
                    None => {
                        if mode == FileMode::Write {
                            let new_inode = Arc::new(Mutex::new(Inode::File(Vec::new())));
                            children.insert(
                                child_path.components.last().unwrap().clone(),
                                new_inode.clone(),
                            );
                            new_inode
                        } else {
                            return Err(FsError::NotFound);
                        }
                    }
                }
            }
        };

        Ok(Box::new(TmpFile {
            path,
            mode,
            offset: 0,
            inode,
        }))
    }

    fn close(&mut self, _file: Box<dyn File>) -> Result<(), FsError> {
        Ok(())
    }

    fn mount(&mut self, _mount_point: &str, _fs: Box<dyn FileSystem>) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
        // Ok(())
    }

    fn unmount(&mut self, _mount_point: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
}
