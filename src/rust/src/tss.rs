use core::arch::asm;

use crate::constant::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};

#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy)]
pub struct Tss {
    link: u32,
    esp0: u32,
    ss0: u32,
    esp1: u32,
    ss1: u32,
    esp2: u32,
    ss2: u32,
    cr3: u32,
    eip: u32,
    eflags: u32,
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    es: u32,
    cs: u32,
    ss: u32,
    ds: u32,
    fs: u32,
    gs: u32,
    ldtr: u32,
    iopb: u32,
    ssp: u32,
}

impl Tss {
    pub fn new_with_kernel_stack(esp0: u32, ss0: u32) -> Self {
        let code_segment = (KERNEL_CODE_SELECTOR | 0x03) as u32;
        let data_segment = (KERNEL_DATA_SELECTOR | 0x03) as u32;
        Self {
            esp0,
            ss0,
            cs: code_segment,
            ds: data_segment,
            es: data_segment,
            fs: data_segment,
            gs: data_segment,
            ss: data_segment,
            ..Default::default()
        }
    }
}

pub unsafe fn ltr(sel: u16) {
    unsafe {
        asm!(
            "ltr {0:x}",
            in(reg) sel,
            options(nostack, preserves_flags)
        );
    }
}
