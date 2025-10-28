use core::arch::asm;

use crate::{device::io::outb, interrupts::idt::Idtr};

pub fn idt_load(idtr: &Idtr) {
    unsafe {
        asm!(
            "lidt [{0}]",
            in(reg) idtr,
            options(nostack, readonly),
        );
    }
}

pub fn get_cr2() -> u32 {
    let cr2: u32;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2);
    }
    cr2
}

pub fn enable_interrupts() {
    unsafe { asm!("sti", options(nostack)) }
}

pub fn disable_interrupts() {
    unsafe { asm!("cli", options(nostack)) }
}

#[inline(always)]
pub fn eoi_pic1() {
    unsafe { outb(0x20, 0x20) };
}

#[inline]
pub fn is_interrupts_enabled() -> bool {
    let flags: u32;
    unsafe {
        core::arch::asm!(
            "pushfd",
            "pop {0}",
            out(reg) flags,
            options(nomem, preserves_flags),
        );
    }
    (flags & (1 << 9)) != 0
}

#[warn(dead_code)]
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let saved_intpt_flag = is_interrupts_enabled();

    if saved_intpt_flag {
        disable_interrupts();
    }

    // do `f` while interrupts are disabled
    let ret = f();

    if saved_intpt_flag {
        enable_interrupts();
    }

    ret
}
