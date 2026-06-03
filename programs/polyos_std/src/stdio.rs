use core::{
    ffi::c_void,
    fmt::{self, Write},
};

const STDIN_FILENO: i32 = 0;
const STDOUT_FILENO: i32 = 1;
const STDERR_FILENO: i32 = 2;

struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_all(STDOUT_FILENO, s.as_bytes()).map_err(|_| fmt::Error)
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
        write_all(STDERR_FILENO, s.as_bytes()).map_err(|_| fmt::Error)
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
        let key = read_byte();
        match key {
            0x08 => {
                if i > 0 {
                    let _ = write_all(STDOUT_FILENO, &[0x08]);
                    buffer[i] = b'\0';
                    i -= 1;
                }
            }
            13 => break,
            k => {
                if output_while_typing {
                    let _ = write_all(STDOUT_FILENO, &[key]);
                }
                buffer[i] = k;
                i += 1;
            }
        }
    }
    i
}

fn read_byte() -> u8 {
    let mut byte = 0_u8;
    loop {
        let read = unsafe {
            crate::bindings::read(
                STDIN_FILENO,
                &mut byte as *mut u8 as *mut c_void,
                core::mem::size_of::<u8>(),
            )
        };

        if read == 1 {
            return byte;
        }

        unsafe {
            crate::bindings::polyos_sleep(1);
        }
    }
}

fn write_all(fd: i32, mut bytes: &[u8]) -> Result<(), ()> {
    while !bytes.is_empty() {
        let written =
            unsafe { crate::bindings::write(fd, bytes.as_ptr() as *const c_void, bytes.len()) };
        if written <= 0 {
            return Err(());
        }

        bytes = &bytes[written as usize..];
    }

    Ok(())
}
