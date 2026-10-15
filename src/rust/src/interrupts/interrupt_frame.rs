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

impl InterruptFrame {
    #[inline]
    pub fn syscall_number(&self) -> u32 {
        self.eax
    }
    #[inline]
    pub fn syscall_arg1(&self) -> u32 {
        self.ebx
    }
    #[inline]
    pub fn syscall_arg2(&self) -> u32 {
        self.ecx
    }
    #[inline]
    pub fn syscall_arg3(&self) -> u32 {
        self.edx
    }
    #[inline]
    pub fn syscall_arg4(&self) -> u32 {
        self.esi
    }
    #[inline]
    pub fn syscall_arg5(&self) -> u32 {
        self.edi
    }
    #[inline]
    pub fn syscall_arg6(&self) -> u32 {
        self.ebp
    }
}
