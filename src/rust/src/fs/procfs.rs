use super::{
    FileHandle, FileMetadata, FileOps, FileSystem, FileSystemDriver, FsError, MountOptions,
};
use crate::kernel::KERNEL;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
#[derive(Debug)]
pub struct ProcFsDriver;
impl FileSystemDriver for ProcFsDriver {
    fn mount(&self, _options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        Ok(Arc::new(ProcFs))
    }
}
struct ProcFs;
fn executable() -> Result<String, FsError> {
    KERNEL
        .with_task_manager(|tm| {
            tm.get_current()
                .map(|t| t.read().process.executable_path.clone())
        })
        .ok_or(FsError::NotFound)
}
impl FileSystem for ProcFs {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        Ok(FileHandle::new(Box::new(TextFile {
            text: contents(path)?,
            offset: 0,
        })))
    }
    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        match path.trim_matches('/') {
            "" => Ok(["self", "uptime", "meminfo", "stat"]
                .iter()
                .map(|s| s.to_string())
                .collect()),
            "self" => Ok(vec!["exe".to_string(), "status".to_string()]),
            _ => Err(FsError::NotADirectory),
        }
    }
    fn read_link(&self, path: &str) -> Result<String, FsError> {
        if path.trim_matches('/') == "self/exe" {
            executable()
        } else {
            Err(FsError::InvalidArgument)
        }
    }
    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        let (mode, size, is_dir) = match path.trim_matches('/') {
            "" | "self" => (0o555, 0, true),
            "self/exe" => (0o120777, executable()?.len() as u64, false),
            _ => (0o444, contents(path)?.len() as u64, false),
        };
        Ok(FileMetadata {
            uid: 0,
            gid: 0,
            mode,
            size,
            is_dir,
        })
    }
    fn create(&self, _path: &str, _directory: bool) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn remove(&self, _path: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn chmod(&self, _path: &str, _mode: u16) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn chown(&self, _path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
}

fn contents(path: &str) -> Result<String, FsError> {
    match path.trim_matches('/') {
        "uptime" => {
            let s = crate::inspection::snapshot();
            Ok(format!(
                "{}.{:03}\n",
                s.uptime(),
                s.ticks % crate::constant::TIMER_HZ as u64
            ))
        }
        "meminfo" => {
            let s = crate::inspection::snapshot();
            Ok(format!(
                "MemTotal: {} kB\nMemFree: {} kB\nMemAvailable: {} kB\n",
                s.total / 1024,
                s.free / 1024,
                s.free / 1024
            ))
        }
        "stat" => {
            let s = crate::inspection::snapshot();
            Ok(format!(
                "processes {}\nthreads {}\ncpus 1\npage_size 4096\nclock_ticks {}\n",
                s.processes,
                s.tasks,
                crate::constant::TIMER_HZ
            ))
        }
        "self/status" => {
            let p = KERNEL
                .with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
                .ok_or(FsError::NotFound)?;
            let threads = KERNEL.with_task_manager(|tm| tm.process_task_count(p.pid));
            Ok(format!(
                "Name:\t{}\nPid:\t{}\nPPid:\t{}\nThreads:\t{}\nUid:\t{}\t{}\nGid:\t{}\t{}\n",
                p.executable_path,
                p.pid,
                p.parent_pid().unwrap_or(0),
                threads,
                p.uid,
                p.euid,
                p.gid,
                p.egid
            ))
        }
        "" | "self" => Err(FsError::IsADirectory),
        _ => Err(FsError::NotFound),
    }
}
struct TextFile {
    text: String,
    offset: usize,
}
impl FileOps for TextFile {
    fn read(&mut self, out: &mut [u8]) -> Result<usize, FsError> {
        let start = self.offset.min(self.text.len());
        let n = out.len().min(self.text.len() - start);
        out[..n].copy_from_slice(&self.text.as_bytes()[start..start + n]);
        self.offset += n;
        Ok(n)
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, FsError> {
        Err(FsError::PermissionDenied)
    }
    fn seek(&mut self, pos: usize) -> Result<usize, FsError> {
        self.offset = pos;
        Ok(pos)
    }
    fn position(&self) -> Result<usize, FsError> {
        Ok(self.offset)
    }
    fn poll(&self, events: u16) -> u16 {
        events & 1
    }
    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(FileMetadata {
            mode: 0o444,
            size: self.text.len() as u64,
            ..Default::default()
        })
    }
}
