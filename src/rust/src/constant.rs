pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

pub const HEAP_SIZE_BYTES: usize = 1024 * 1024 * 100; // 100MB
pub const HEAP_ADDRESS: usize = 0x01000000;

pub const PAGING_PAGE_SIZE_BIT: usize = 12;
pub const PAGING_PAGE_SIZE: usize = 1 << PAGING_PAGE_SIZE_BIT;
pub const PAGING_PAGE_TABLE_SIZE_BIT: usize = 10;
pub const PAGING_PAGE_TABLE_SIZE: usize = 1 << PAGING_PAGE_TABLE_SIZE_BIT;

pub const MAX_PATH: usize = 256;

pub const TOTAL_GDT_SEGMENTS: usize = 6;

pub const PROGRAM_VIRTUAL_ADDRESS: usize = 0x00400000;
pub const USER_HEAP_START: usize = 0x00800000;
pub const USER_HEAP_END: usize = 0x01000000;
pub const USER_PROGRAM_STACK_SIZE: usize = 1024 * 16; // 16KB
pub const USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START: usize = 0x003FF000;
pub const USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END: usize =
    USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START - USER_PROGRAM_STACK_SIZE;

pub const USER_DATA_SEGMENT: u32 = 0x23;
pub const USER_CODE_SEGMENT: u32 = 0x1B;

pub const PIC_MASTER_COMMAND_PORT: u16 = 0x20;
pub const PIC_MASTER_DATA_PORT: u16 = 0x21;
pub const PIC_SLAVE_COMMAND_PORT: u16 = 0xA0;
pub const PIC_SLAVE_DATA_PORT: u16 = 0xA1;
pub const PIC_MASTER_VECTOR_OFFSET: u16 = 0x20;
pub const PIC_SLAVE_VECTOR_OFFSET: u16 = 0x28;
pub const PIC_SLAVE_IRQ_LINE: u8 = 2;
pub const PIC_SLAVE_IRQ_MASK: u8 = 1u8 << PIC_SLAVE_IRQ_LINE;

pub const PIT_BASE_FREQUENCY_HZ: u32 = 1_193_182;
pub const TIMER_HZ: u32 = 1000;

pub const fn irq_to_vector(irq_line: u8) -> Option<u16> {
    if irq_line < 16 {
        Some(PIC_MASTER_VECTOR_OFFSET + irq_line as u16)
    } else {
        None
    }
}
