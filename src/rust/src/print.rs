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

pub fn terminal_writechar(c: u8, color: u8) {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.write_char_color(c, color.into());
        } else {
            serial_println!("Terminal write char not supported in this mode");
        }
    });
}

pub fn clear_screen() {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.clear(ScreenChar::new(
                b' ',
                ColorCode::new(Color::White, Color::Black),
            ));
        }
    });
}

pub fn set_color(background: Color, foreground: Color) {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            let color = ColorCode::new(foreground, background);
            text.set_color(color);
        }
    });
}

pub fn disable_cursor() {
    KERNEL.with_text(|text| {
        if let Some(text) = text {
            text.disable_cursor();
        }
    });
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
