use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{ffi::CStr, mem};

pub const O_RDONLY: i32 = crate::bindings::O_RDONLY as i32;
pub const O_WRONLY: i32 = crate::bindings::O_WRONLY as i32;
pub const O_RDWR: i32 = crate::bindings::O_RDWR as i32;
pub const O_CREAT: i32 = crate::bindings::O_CREAT as i32;
pub const O_TRUNC: i32 = crate::bindings::O_TRUNC as i32;
pub const O_APPEND: i32 = crate::bindings::O_APPEND as i32;
pub const O_NONBLOCK: i32 = crate::bindings::O_NONBLOCK as i32;

pub const DT_DIR: u8 = crate::bindings::DT_DIR as u8;
pub const DT_REG: u8 = crate::bindings::DT_REG as u8;

#[derive(Clone, Debug)]
pub struct FileStat {
    pub size: i32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub is_dir: bool,
}

impl FileStat {
    pub fn is_dir(&self) -> bool {
        self.mode & crate::bindings::S_IFMT == crate::bindings::S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        self.mode & crate::bindings::S_IFMT == crate::bindings::S_IFREG
    }
}

#[derive(Clone, Debug)]
pub struct DirEntry {
    pub name: String,
    pub file_type: u8,
}

pub fn errno() -> i32 {
    unsafe { crate::bindings::errno }
}

pub fn open(path: &str, flags: i32, mode: i32) -> Result<i32, i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::open(path.as_ptr() as *const i8, flags, mode) };
    if result >= 0 {
        Ok(result)
    } else {
        Err(errno())
    }
}

pub fn close(fd: i32) -> Result<(), i32> {
    let result = unsafe { crate::bindings::close(fd) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn stat(path: &str) -> Result<FileStat, i32> {
    stat_with(path, crate::bindings::stat)
}

pub fn lstat(path: &str) -> Result<FileStat, i32> {
    stat_with(path, crate::bindings::lstat)
}

pub fn mkdir(path: &str, mode: i32) -> Result<(), i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::mkdir(path.as_ptr() as *const i8, mode) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn rmdir(path: &str) -> Result<(), i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::rmdir(path.as_ptr() as *const i8) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn unlink(path: &str) -> Result<(), i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::unlink(path.as_ptr() as *const i8) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn chmod(path: &str, mode: i32) -> Result<(), i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::chmod(path.as_ptr() as *const i8, mode) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn chown(path: &str, uid: u32, gid: u32) -> Result<(), i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::chown(path.as_ptr() as *const i8, uid, gid) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn umask(mask: i32) -> i32 {
    unsafe { crate::bindings::umask(mask) }
}

pub fn chdir(path: &str) -> Result<(), i32> {
    let path = nul_terminated(path);
    let result = unsafe { crate::bindings::chdir(path.as_ptr() as *const i8) };
    if result == 0 {
        Ok(())
    } else {
        Err(errno())
    }
}

pub fn getcwd() -> Result<String, i32> {
    let mut buffer = [0_i8; 256];
    let result = unsafe { crate::bindings::getcwd(buffer.as_mut_ptr(), buffer.len()) };
    if result.is_null() {
        return Err(errno());
    }

    let cwd = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    Ok(cwd.to_str().unwrap_or("").to_string())
}

pub fn read_dir(path: &str) -> Result<Vec<DirEntry>, i32> {
    let fd = open(path, O_RDONLY, 0)?;
    let mut all = Vec::new();

    loop {
        let mut entries: [crate::bindings::dirent; 16] = unsafe { mem::zeroed() };
        let bytes = unsafe {
            crate::bindings::getdents(
                fd,
                entries.as_mut_ptr(),
                mem::size_of_val(&entries),
            )
        };

        if bytes < 0 {
            let error = errno();
            let _ = close(fd);
            return Err(error);
        }

        if bytes == 0 {
            break;
        }

        let count = bytes as usize / mem::size_of::<crate::bindings::dirent>();
        for entry in entries.iter().take(count) {
            all.push(DirEntry {
                name: dirent_name(entry),
                file_type: entry.d_type,
            });
        }
    }

    close(fd)?;
    Ok(all)
}

fn stat_with(
    path: &str,
    call: unsafe extern "C" fn(*const i8, *mut crate::bindings::file_stat) -> i32,
) -> Result<FileStat, i32> {
    let path = nul_terminated(path);
    let mut stat: crate::bindings::file_stat = unsafe { mem::zeroed() };
    let result = unsafe { call(path.as_ptr() as *const i8, &mut stat) };
    if result != 0 {
        return Err(errno());
    }

    Ok(FileStat {
        size: stat.size,
        mode: stat.mode,
        uid: stat.uid,
        gid: stat.gid,
        is_dir: stat.is_dir != 0,
    })
}

fn dirent_name(entry: &crate::bindings::dirent) -> String {
    let bytes = unsafe {
        core::slice::from_raw_parts(
            entry.d_name.as_ptr() as *const u8,
            entry.d_name.len(),
        )
    };
    let len = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..len])
        .unwrap_or("")
        .to_string()
}

fn nul_terminated(value: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(value.len() + 1);
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
    bytes
}
