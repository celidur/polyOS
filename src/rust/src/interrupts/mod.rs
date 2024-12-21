use crate::bindings::{are_interrupts_enabled, disable_interrupts, enable_interrupts};

#[warn(dead_code)]
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    // true if the interrupt flag is set (i.e. interrupts are enabled)
    let saved_intpt_flag = unsafe { are_interrupts_enabled() } & 200 != 0;

    // if interrupts are enabled, disable them for now
    if saved_intpt_flag {
        unsafe { disable_interrupts() };
    }

    // do `f` while interrupts are disabled
    let ret = f();

    // re-enable interrupts if they were previously enabled
    if saved_intpt_flag {
        unsafe { enable_interrupts() };
    }

    // return the result of `f` to the caller
    ret
}
