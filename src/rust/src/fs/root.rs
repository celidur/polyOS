use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

use super::{path::Path, FileSystem};

lazy_static! {
    pub static ref ROOT_FS: Mutex<RootFs> = Mutex::new(RootFs::new());
}

#[derive(Debug)]
pub struct RootFs {
    mounts: BTreeMap<String, Box<dyn FileSystem>>,
}

impl RootFs {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
        }
    }
}

impl FileSystem for RootFs {
    fn open(
        &mut self,
        path: &str,
        mode: super::FileMode,
    ) -> Result<Box<dyn super::File>, super::FsError> {
        let path = Path::new(path);

        for i in 0..path.components.len() {
            let parent = format!(
                "/{}",
                path.components
                    .iter()
                    .take(i)
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join("/")
            );
            if let Some(fs) = self.mounts.get_mut(&parent) {
                return fs.open(&path.as_string(), mode);
            }
        }

        Err(super::FsError::NotFound)
    }

    fn close(&mut self, file: Box<dyn super::File>) -> Result<(), super::FsError> {
        let path = file.path();

        for i in 0..path.components.len() {
            let parent = format!(
                "{}/",
                path.components
                    .iter()
                    .take(i)
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join("/")
            );
            if let Some(fs) = self.mounts.get_mut(&parent) {
                fs.close(file)?;
                return Ok(());
            }
        }

        Err(super::FsError::NotFound)
    }

    fn mount(&mut self, mount_point: &str, fs: Box<dyn FileSystem>) -> Result<(), super::FsError> {
        let path = Path::new(mount_point);
        let mut fs = fs;
        if let Some(parent_fs) = self.mounts.get_mut(&path.parent().as_string()) {
            return parent_fs.mount(&path.as_string(), fs);
        }

        if self.mounts.contains_key(&path.as_string()) {
            return Err(super::FsError::AlreadyMounted);
        }

        if path.parent().as_string() != "/" {
            return Err(super::FsError::InvalidArgument);
        }

        let mut to_remove = Vec::new();
        for (mount_point, _) in self.mounts.iter() {
            if Path::new(mount_point).parent().as_string() == path.as_string() {
                to_remove.push(mount_point.clone());
            }
        }

        for mount_point in to_remove {
            let child_fs = self.mounts.remove(&mount_point).unwrap();
            // todo add error handling
            let _ = fs.mount(&mount_point, child_fs);
        }

        self.mounts.insert(path.as_string(), fs);

        Ok(())
    }

    fn unmount(&mut self, mount_point: &str) -> Result<(), super::FsError> {
        let path = Path::new(mount_point);

        if let Some(fs) = self.mounts.get_mut(&path.as_string()) {
            return fs.unmount(&path.as_string());
        }

        self.mounts
            .remove(mount_point)
            .ok_or(super::FsError::NotFound)?;

        Ok(())
    }
}
