#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FileStat {
    pub size: u32,
    pub flags: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub is_dir: u32,
}
