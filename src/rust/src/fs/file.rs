use alloc::{boxed::Box, fmt};
use bitflags::bitflags;
use core::fmt::Debug;

use super::path::Path;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FileStatFlags: u32 {
        const IS_FILE    = 0b0000;
        const READ_ONLY   = 0b0001;
    }
}

#[derive(Debug)]
pub enum FsError {
    NotFound,
    PermissionDenied,
    DiskError,
    InvalidArgument,
    AlreadyMounted,
    MountPointInUse,
    Unknown,
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct FileStat {
    pub size: u32,
    pub flags: FileStatFlags,
}

pub enum SeekMode {
    Set,
    Cur,
    End,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileMode {
    Read,
    Write,
    Append,
}

pub trait FileSystem: Debug + Send + Sync {
    fn open(&mut self, path: &str, mode: FileMode) -> Result<Box<dyn File>, FsError>;
    fn close(&mut self, file: Box<dyn File>) -> Result<(), FsError>;
    fn mount(&mut self, mount_point: &str, fs: Box<dyn FileSystem>) -> Result<(), FsError>;
    fn unmount(&mut self, mount_point: &str) -> Result<(), FsError>;
}

pub trait File {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FsError>;
    fn write(&mut self, buffer: &[u8]) -> Result<usize, FsError>;
    fn seek(&mut self, offset: u64, mode: SeekMode) -> Result<(), FsError>;
    fn stat(&self) -> Result<FileStat, FsError>;
    fn get_position(&self) -> u64;
    fn get_size(&self) -> u64;
    fn path(&self) -> &Path;
}
