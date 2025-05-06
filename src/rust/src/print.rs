use alloc::format;

use crate::{
    device::screen::{Color, ColorCode, ScreenChar},
    kernel::KERNEL,
    serial_println,
};

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;

    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.write_fmt(args).expect("Printing to screen failed");
        } else {
            serial_println!("Terminal print not supported in this mode");
        }
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[unsafe(no_mangle)]
pub extern "C" fn print(buf: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    print!("{}", unsafe {
        core::ffi::CStr::from_ptr(buf).to_string_lossy()
    });

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn print_c(buf: *const ::core::ffi::c_char, color: u8) -> ::core::ffi::c_int {
    let res = format!("{}", unsafe {
        core::ffi::CStr::from_ptr(buf).to_string_lossy()
    });

    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.write_str_color(res.as_str(), color.into());
        } else {
            serial_println!("Terminal print not supported in this mode");
        }
    });

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_writechar(c: u8, color: u8) -> ::core::ffi::c_int {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.write_char_color(c, color.into());
        } else {
            serial_println!("Terminal write char not supported in this mode");
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_backspace() -> ::core::ffi::c_int {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.backspace();
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn clear_screen() -> ::core::ffi::c_int {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.clear(ScreenChar::new(
                b' ',
                ColorCode::new(Color::White, Color::Black),
            ));
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn set_color(background: u8, foreground: u8) -> ::core::ffi::c_int {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            let color = ColorCode::new(Color::from(foreground), Color::from(background));
            text.set_color(color);
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn disable_cursor() -> ::core::ffi::c_int {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.disable_cursor();
        }
    });
    0
}
