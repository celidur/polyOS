use crate::kernel::KERNEL;

#[doc(hidden)]
pub fn _serial(args: ::core::fmt::Arguments) {
    KERNEL.serial(args);
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_serial(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[unsafe(no_mangle)]
pub extern "C" fn serial_write(buf: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    serial_print!("{}", unsafe {
        core::ffi::CStr::from_ptr(buf).to_string_lossy()
    });
    
    0
}
