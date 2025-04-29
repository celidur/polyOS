use crate::{bindings::strlen, kernel::KERNEL};

#[doc(hidden)]
pub fn _serial(args: ::core::fmt::Arguments) {
    use crate::interrupts;
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        KERNEL
            .serial_port
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed")
    });
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
    use crate::interrupts;
    use core::fmt::Write;

    let len = unsafe { strlen(buf) as usize };

    interrupts::without_interrupts(|| {
        use core::str;
        KERNEL
            .serial_port
            .lock()
            .write_str(unsafe { str::from_raw_parts(buf as *const u8, len) })
            .expect("Printing to serial failed")
    });

    0
}
