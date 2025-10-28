use core::arch::asm;
use lazy_static::lazy_static;
use spin::RwLock;

use crate::{
    constant::TOTAL_GDT_SEGMENTS,
    tss::{Tss, ltr},
};

const TYPE_KCODE: u8 = 0x9A;
const TYPE_KDATA: u8 = 0x92;
const TYPE_UCODE: u8 = 0xFA;
const TYPE_UDATA: u8 = 0xF2;
const TYPE_TSS: u8 = 0xE9;

// selectors (index * 8) | RPL
const KERNEL_CODE_SELECTOR: u16 = (1 * 8) | 0;
const KERNEL_DATA_SELECTOR: u16 = (2 * 8) | 0;
// const USER_CODE_SELECTOR: u16 = (3 * 8) | 3;
// const USER_DATA_SELECTOR: u16 = (4 * 8) | 3;
const TSS_SELECTOR: u16 = (5 * 8) | 0;

lazy_static! {
    pub static ref GDT: RwLock<Gdt> = RwLock::new(Gdt::new());
}

#[derive(Debug)]
pub struct Gdt {
    pub entries: [GdtEntryRaw; TOTAL_GDT_SEGMENTS],
    tss: Tss,
}

impl Gdt {
    pub fn new() -> Self {
        let tss = Tss::new_with_kernel_stack(0x600000, KERNEL_DATA_SELECTOR as u32);

        Self {
            entries: [GdtEntryRaw::encode_from(0x00, 0x00, 0x00, 0x00); TOTAL_GDT_SEGMENTS],
            tss,
        }
    }

    pub fn init_gdt(&mut self) {
        self.entries[1] = GdtEntryRaw::encode_from(0x00, 0xFFFFFFFF, TYPE_KCODE, 0xCF); // kernel code segment
        self.entries[2] = GdtEntryRaw::encode_from(0x00, 0xFFFFFFFF, TYPE_KDATA, 0xCF); // kernel data segment
        self.entries[3] = GdtEntryRaw::encode_from(0x00, 0xFFFFFFFF, TYPE_UCODE, 0xCF); // user code segment
        self.entries[4] = GdtEntryRaw::encode_from(0x00, 0xFFFFFFFF, TYPE_UDATA, 0xCF); // user data segment

        let tss_base = (&self.tss as *const _ as usize) as u32;
        let tss_limit = tss_base + core::mem::size_of::<Tss>() as u32;
        self.entries[5] = GdtEntryRaw::encode_from(tss_base, tss_limit, TYPE_TSS, 0x00); // TSS segment

        let gdt_ptr = GdtDescriptor {
            limit: (core::mem::size_of::<GdtEntryRaw>() * self.entries.len() - 1) as u16,
            base: &self.entries as *const _ as u32,
        };

        unsafe {
            lgdt(&gdt_ptr);

            // Far jump to reload CS from the new GDT
            asm!(
                "push {code_sel}",
                "lea eax, [2f]",
                "push eax",
                "retf",
                "2:",
                code_sel = const KERNEL_CODE_SELECTOR as u32,
                options(nostack, preserves_flags)
            );

            // Reload data segments
            load_data_segs(KERNEL_DATA_SELECTOR);

            ltr(TSS_SELECTOR);
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default, Debug)]
pub struct GdtEntryRaw {
    limit: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    flags: u8,
    base_hi: u8,
}

impl GdtEntryRaw {
    fn encode_from(base: u32, limit: u32, ty: u8, gran: u8) -> GdtEntryRaw {
        let base_low = (base & 0xFFFF) as u16;
        let base_mid = ((base >> 16) & 0xFF) as u8;
        let base_hi = ((base >> 24) & 0xFF) as u8;

        let flags = ((limit >> 16) & 0x0F) as u8 | (gran & 0xF0);
        let limit = (limit & 0xFFFF) as u16;

        let access = ty;

        GdtEntryRaw {
            limit,
            base_low,
            base_mid,
            access,
            flags,
            base_hi,
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default, Debug)]
struct GdtDescriptor {
    limit: u16,
    base: u32,
}

unsafe fn lgdt(desc: &GdtDescriptor) {
    unsafe {
        asm!(
            "lgdt [{0}]",
            in(reg) desc,
            options(nostack, preserves_flags)
        );
    }
}

#[inline(always)]
unsafe fn load_data_segs(sel: u16) {
    unsafe {
        asm!(
            "mov ds, {0:x}",
            "mov es, {0:x}",
            "mov fs, {0:x}",
            "mov gs, {0:x}",
            "mov ss, {0:x}",
            in(reg) sel,
            options(nostack, preserves_flags)
        );
    }
}
