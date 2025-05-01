use crate::bindings::{disable_interrupts, enable_interrupts};

#[inline]
pub fn interrupts_enabled() -> bool {
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
    let saved_intpt_flag = interrupts_enabled();

    if saved_intpt_flag {
        unsafe { disable_interrupts() };
    }

    // do `f` while interrupts are disabled
    let ret = f();

    if saved_intpt_flag {
        unsafe { enable_interrupts() };
    }

    ret
}
