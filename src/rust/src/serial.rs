use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

use crate::bindings::strlen;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[doc(hidden)]
pub fn _serial(args: ::core::fmt::Arguments) {
    use crate::interrupts;
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed")
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_serial(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[no_mangle]
pub extern "C" fn serial_write(buf: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    use crate::interrupts;
    use core::fmt::Write;

    let len = unsafe {
        strlen(buf) as usize
    };

    interrupts::without_interrupts(|| {
        use core::str;
        SERIAL1
            .lock()
            .write_str(unsafe{ str::from_raw_parts(buf as *const u8, len)})
            .expect("Printing to serial failed")
    });

    0
}
