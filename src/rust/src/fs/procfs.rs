//! Read-only system and live-process views; each open retains a byte snapshot.
use super::{
    FileHandle, FileMetadata, FileOps, FileSystem, FileSystemDriver, FsError, MountOptions,
};
use crate::{kernel::KERNEL, schedule::process::Process};
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
fn process(selector: &str) -> Result<Arc<Process>, FsError> {
    if selector == "self" {
        return KERNEL
            .with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
            .ok_or(FsError::NotFound);
    }
    if selector.is_empty() || !selector.bytes().all(|b| b.is_ascii_digit()) {
        return Err(FsError::NotFound);
    }
    let pid = selector.parse::<u32>().map_err(|_| FsError::NotFound)?;
    KERNEL
        .with_process_manager(|pm| pm.get(pid))
        .ok_or(FsError::NotFound)
}
fn sensitive(target: &Process) -> Result<(), FsError> {
    let caller = KERNEL
        .with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
        .ok_or(FsError::PermissionDenied)?;
    if caller.euid() == 0 || (caller.euid() == target.uid() && caller.euid() == target.euid()) {
        Ok(())
    } else {
        Err(FsError::PermissionDenied)
    }
}
fn child_path(path: &str) -> Option<(&str, &str)> {
    path.split_once('/')
}
impl FileSystem for ProcFs {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let path = path.trim_matches('/');
        let text = contents(path)?;
        let metadata = metadata(path, text.len() as u64)?;
        Ok(FileHandle::new(Box::new(TextFile {
            text,
            metadata,
            offset: 0,
        })))
    }
    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            let mut entries: Vec<String> =
                ["self", "uptime", "meminfo", "stat", "metrics", "timing"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            entries.extend(
                KERNEL
                    .with_process_manager(|pm| pm.live_processes())
                    .into_iter()
                    .map(|p| p.pid.to_string()),
            );
            return Ok(entries);
        }
        process(path)?;
        Ok(["exe", "status", "cmdline", "maps", "stat"]
            .iter()
            .map(|s| s.to_string())
            .collect())
    }
    fn read_link(&self, path: &str) -> Result<String, FsError> {
        let path = path.trim_matches('/');
        if let Some((selector, "exe")) = child_path(path) {
            let p = process(selector)?;
            sensitive(&p)?;
            return Ok(p.executable_path.clone());
        }
        Err(FsError::InvalidArgument)
    }
    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok(FileMetadata {
                mode: 0o555,
                is_dir: true,
                ..Default::default()
            });
        }
        if !path.contains('/') && (path == "self" || path.bytes().all(|b| b.is_ascii_digit())) {
            let p = process(path)?;
            return Ok(FileMetadata {
                uid: p.uid(),
                gid: p.gid(),
                mode: 0o555,
                is_dir: true,
                ..Default::default()
            });
        }
        if let Some((selector, "exe")) = child_path(path) {
            let p = process(selector)?;
            return Ok(FileMetadata {
                uid: p.uid(),
                gid: p.gid(),
                mode: 0o120777,
                size: p.executable_path.len() as u64,
                ..Default::default()
            });
        }
        metadata(path, contents(path)?.len() as u64)
    }
    fn create(&self, _: &str, _: bool) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn remove(&self, _: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn chmod(&self, _: &str, _: u16) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
    fn chown(&self, _: &str, _: u32, _: u32) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
}
fn metadata(path: &str, size: u64) -> Result<FileMetadata, FsError> {
    let mut meta = FileMetadata {
        mode: 0o444,
        size,
        ..Default::default()
    };
    if let Some((selector, leaf)) = child_path(path) {
        let p = process(selector)?;
        meta.uid = p.uid();
        meta.gid = p.gid();
        if matches!(leaf, "cmdline" | "maps") {
            meta.mode = 0o400;
        }
    }
    Ok(meta)
}
fn contents(path: &str) -> Result<String, FsError> {
    if let Some((selector, leaf)) = child_path(path) {
        let p = process(selector)?;
        return process_contents(&p, leaf);
    }
    let s = crate::inspection::snapshot();
    match path {
        "uptime" => Ok(format!(
            "{}.{:03}\n",
            s.uptime(),
            s.ticks % crate::constant::TIMER_HZ as u64
        )),
        "meminfo" => Ok(format!(
            "MemTotal: {} kB\nMemFree: {} kB\nMemAvailable: {} kB\n",
            s.total / 1024,
            s.free / 1024,
            s.free / 1024
        )),
        "stat" => Ok(format!(
            "processes {}\nthreads {}\ncpus 1\npage_size 4096\nclock_ticks {}\n",
            s.processes,
            s.tasks,
            crate::constant::TIMER_HZ
        )),
        "metrics" => Ok(crate::metrics::render(false)),
        "timing" => Ok(crate::metrics::render(true)),
        "" | "self" => Err(FsError::IsADirectory),
        selector if selector.bytes().all(|b| b.is_ascii_digit()) => {
            process(selector)?;
            Err(FsError::IsADirectory)
        }
        _ => Err(FsError::NotFound),
    }
}
fn process_contents(p: &Process, leaf: &str) -> Result<String, FsError> {
    if matches!(leaf, "cmdline" | "maps") {
        sensitive(p)?;
    }
    let (threads, ticks, state, name) = KERNEL.with_task_manager(|tm| tm.inspection(p.pid));
    if threads == 0 {
        return Err(FsError::NotFound);
    }
    let name = String::from_utf8_lossy(&name)
        .trim_end_matches('\0')
        .replace(['\n', '\r', ')'], "_");
    match leaf {
        "cmdline" => {
            let mut out = String::new();
            for arg in &p.command_line {
                out.push_str(arg);
                out.push('\0');
            }
            Ok(out)
        }
        "maps" => {
            let mut out = String::new();
            for (start, end, prot, offset, name) in p.inspection_maps() {
                out.push_str(&format!(
                    "{:08x}-{:08x} {}{}{}p {:08x} 00:00 0 {}\n",
                    start,
                    end,
                    if prot & 1 != 0 { 'r' } else { '-' },
                    if prot & 2 != 0 { 'w' } else { '-' },
                    if prot & 4 != 0 { 'x' } else { '-' },
                    offset,
                    name
                ));
            }
            Ok(out)
        }
        "status" => {
            let c = p.control.lock();
            let ids = &c.credentials;
            let groups = ids
                .groups
                .iter()
                .map(|id| format!("{} ", id))
                .collect::<String>();
            Ok(format!("Name:\t{}\nState:\t{}\nPid:\t{}\nPPid:\t{}\nThreads:\t{}\nUid:\t{}\t{}\t{}\t{}\nGid:\t{}\t{}\t{}\t{}\nGroups:\t{}\n", name, state, p.pid, p.parent_pid().unwrap_or(0), threads,
                ids.uid, ids.euid, ids.saved_uid, ids.euid, ids.gid, ids.egid, ids.saved_gid, ids.egid, groups))
        }
        "stat" => {
            let c = p.control.lock();
            let mut fields = [0u64; 49];
            fields[0] = p.parent_pid().unwrap_or(0) as u64;
            fields[1] = c.pgid as u64;
            fields[2] = c.sid as u64;
            fields[10] = ticks;
            fields[14] = 20;
            fields[16] = threads as u64;
            fields[18] = p.start_tick;
            let maps = p.inspection_maps();
            fields[19] = maps.iter().map(|(a, b, _, _, _)| (b - a) as u64).sum();
            fields[20] = fields[19] / 4096;
            fields[21] = u32::MAX as u64;
            let mut out = format!("{} ({}) {}", p.pid, name, state);
            for value in fields {
                out.push_str(&format!(" {}", value));
            }
            out.push('\n');
            Ok(out)
        }
        "exe" => Err(FsError::InvalidArgument),
        _ => Err(FsError::NotFound),
    }
}
struct TextFile {
    text: String,
    metadata: FileMetadata,
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
    fn write(&mut self, _: &[u8]) -> Result<usize, FsError> {
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
        Ok(self.metadata.clone())
    }
}
