use core::fmt::{self, Write};

use crate::bindings::{polyos_getkeyblock, polyos_putchar, remove_last_char};

struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut start = 0;
        let bytes = s.as_bytes();
        let buffer_size = 1024;
        let mut buffer = [0u8; 1024]; // Adjust size as needed

        while start < bytes.len() {
            let len = if bytes.len() - start >= buffer_size - 1 {
                buffer_size - 1
            } else {
                bytes.len() - start
            };

            buffer[..len].copy_from_slice(&bytes[start..start + len]);

            buffer[len] = 0;

            unsafe {
                crate::bindings::print(buffer.as_ptr() as *mut i8);
            }

            start += len;
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    let mut writer = ConsoleWriter;
    writer.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::stdio::print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}

struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut start = 0;
        let bytes = s.as_bytes();
        let buffer_size = 1024;
        let mut buffer = [0u8; 1024]; // Adjust size as needed

        while start < bytes.len() {
            let len = if bytes.len() - start >= buffer_size - 1 {
                buffer_size - 1
            } else {
                bytes.len() - start
            };

            buffer[..len].copy_from_slice(&bytes[start..start + len]);

            buffer[len] = 0;

            unsafe {
                crate::bindings::serial(buffer.as_ptr() as *mut i8);
            }

            start += len;
        }
        Ok(())
    }
}

pub fn serial_print(args: fmt::Arguments) {
    let mut writer = SerialWriter;
    writer.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::stdio::serial_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", format_args!($($arg)*));
    };
}

pub fn terminal_readline(buffer: &mut [u8], output_while_typing: bool) -> usize {
    let mut i = 0;
    while i < buffer.len() {
        let key = unsafe { polyos_getkeyblock() };
        match key {
            0x08 => {
                if i > 0 {
                    unsafe { remove_last_char() };
                    buffer[i] = b'\0';
                    i -= 1;
                }
            }
            13 => break,
            k => {
                if output_while_typing {
                    unsafe { polyos_putchar(key.try_into().unwrap()) };
                }
                buffer[i] = k.try_into().unwrap();
                i += 1;
            }
        }
    }
    i
}
