#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FileStat {
    pub size: u32,
    pub flags: u32,
}
