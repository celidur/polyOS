#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RawDirEntry {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub attr: u8,
    pub nt_res: u8,
    pub create_time_fine: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub access_date: u16,
    pub cluster_high: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub cluster_low: u16,
    pub file_size: u32,
}

impl RawDirEntry {
    pub fn is_free(&self) -> bool {
        self.name[0] == 0x00 || self.name[0] == 0xE5
    }
    pub fn is_dir(&self) -> bool {
        self.attr & 0x10 != 0
    }
    pub fn is_lfn(&self) -> bool {
        self.attr == 0x0F
    }

    pub fn name_and_ext_byte(&self, i: usize) -> u8 {
        if i < 8 { self.name[i] } else { self.ext[i - 8] }
    }
}
