#[doc(hidden)]
pub fn _serial(args: ::core::fmt::Arguments) {
    crate::device::serial::SERIAL_DRIVER.write(args);
}
