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
    let _ = writer.write_fmt(args);
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
    let _ = writer.write_fmt(args);
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
    if buffer.is_empty() {
        return 0;
    }
    unsafe {
        crate::bindings::polyos_terminal_readline(
            buffer.as_mut_ptr() as *mut i8,
            buffer.len() as i32,
            output_while_typing,
        );
    }
    buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len())
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
