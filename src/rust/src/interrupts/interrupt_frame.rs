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
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptErrorFrame {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub reserved: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,

    pub error_code: u32,
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
    pub esp: u32,
    pub ss: u32,
}

impl InterruptErrorFrame {
    pub fn without_error_code(&self) -> InterruptFrame {
        InterruptFrame {
            edi: self.edi,
            esi: self.esi,
            ebp: self.ebp,
            reserved: self.reserved,
            ebx: self.ebx,
            edx: self.edx,
            ecx: self.ecx,
            eax: self.eax,
            ip: self.ip,
            cs: self.cs,
            flags: self.flags,
            esp: if self.cs & 3 == 3 { self.esp } else { 0 },
            ss: if self.cs & 3 == 3 { self.ss } else { 0 },
        }
    }
}

impl InterruptFrame {
    pub fn is_user_mode(&self) -> bool {
        self.cs & 3 == 3
    }
}
