#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptFrame {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub reserved: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,

    pub ip: u32, // instruction pointer
    pub cs: u32,
    pub flags: u32,
    pub esp: u32,
    pub ss: u32,
}
