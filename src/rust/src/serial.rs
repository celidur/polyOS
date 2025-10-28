use crate::kernel::KERNEL;

#[doc(hidden)]
pub fn _serial(args: ::core::fmt::Arguments) {
    KERNEL.serial(args);
}
