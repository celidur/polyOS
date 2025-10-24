pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

pub const TOTAL_INTERRUPTS: usize = 512;

pub const HEAP_SIZE_BYTES: usize = 1024 * 1024 * 100; // 100MB
pub const HEAP_SIZE_BLOCKS: usize = 4096;
pub const HEAP_ADDRESS: usize = 0x01000000;
pub const HEAP_TABLE_ADDRESS: usize = 0x00007E00;

pub const PAGING_PAGE_SIZE: usize = 4096;
pub const PAGING_PAGE_TABLE_SIZE: usize = 1024;

pub const SECTOR_SIZE: usize = 512;

pub const MAX_PATH: usize = 256;
pub const MAX_FILENAME: usize = 256;

pub const MAX_FILESYSTEMS: usize = 12;
pub const MAX_FILE_DESCRIPTORS: usize = 512;

pub const TOTAL_GDT_SEGMENTS: usize = 6;

pub const PROGRAM_VIRTUAL_ADDRESS: usize = 0x00400000;
pub const USER_PROGRAM_STACK_SIZE: usize = 1024 * 16; // 16KB
pub const USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START: usize = 0x003FF000;
pub const USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END: usize =
    USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START - USER_PROGRAM_STACK_SIZE;

pub const USER_DATA_SEGMENT: u32 = 0x23;
pub const USER_CODE_SEGMENT: u32 = 0x1B;

pub const MAX_PROGRAM_ALLOCATIONS: usize = 1024;
pub const MAX_PROCESS: usize = 12;

pub const MAX_INT80H_COMMANDS: usize = 1024;
