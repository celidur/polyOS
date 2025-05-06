use crate::{
    device::screen::{Color, ColorCode, ScreenChar, Vga},
    interrupts,
    kernel::KERNEL,
    serial_println,
};

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use crate::interrupts;
    use core::fmt::Write;

    interrupts::without_interrupts(|| match &mut *KERNEL.vga.write() {
        Vga::Text(text) => {
            text.write_fmt(args).expect("Printing to screen failed");
        }
        _ => {
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
pub extern "C" fn print_c(buf: *const ::core::ffi::c_char, _color: u8) -> ::core::ffi::c_int {
    print!("{}", unsafe {
        core::ffi::CStr::from_ptr(buf).to_string_lossy()
    });

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_writechar(c: u8, color: u8) -> ::core::ffi::c_int {
    interrupts::without_interrupts(|| match &mut *KERNEL.vga.write() {
        Vga::Text(text) => {
            text.write_char_color(c, color.into());
        }
        _ => {
            serial_println!("Terminal write char not supported in this mode");
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_backspace() -> ::core::ffi::c_int {
    interrupts::without_interrupts(|| match &mut *KERNEL.vga.write() {
        Vga::Text(text) => {
            text.backspace();
        }
        _ => {
            serial_println!("Terminal write char not supported in this mode");
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn clear_screen() -> ::core::ffi::c_int {
    interrupts::without_interrupts(|| match &mut *KERNEL.vga.write() {
        Vga::Text(text) => {
            text.clear(ScreenChar::new(
                b' ',
                ColorCode::new(Color::White, Color::Black),
            ));
        }
        _ => {
            serial_println!("Terminal write char not supported in this mode");
        }
    });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn set_color(background: u8, foreground: u8) -> ::core::ffi::c_int {
    match &mut *KERNEL.vga.write() {
        Vga::Text(text) => {
            let color = ColorCode::new(Color::from(foreground), Color::from(background));
            text.set_color(color);
        }
        _ => {
            serial_println!("Terminal write char not supported in this mode");
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn disable_cursor() -> ::core::ffi::c_int {
    match &mut *KERNEL.vga.write() {
        Vga::Text(text) => {
            text.disable_cursor();
        }
        _ => {
            serial_println!("Terminal write char not supported in this mode");
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn set_pixel(x: u32, y: u32, color: u8) -> ::core::ffi::c_int {
    match &mut *KERNEL.vga.write() {
        Vga::Graphic(graphic) => {
            graphic.set_pixel(x, y, color);
        }
        _ => {
            serial_println!("Set pixel not supported in this mode");
        }
    }
    0
}
