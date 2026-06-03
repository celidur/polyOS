use crate::{
    device::screen::{Color, ColorCode, ScreenChar},
    device::screen::SCREEN_DRIVER,
};

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;

    SCREEN_DRIVER.with_text(|text| {
        if let Some(text) = text {
            text.write_fmt(args).expect("Printing to screen failed");
        } else {
            serial_println!("Terminal print not supported in this mode");
        }
    });
}

pub fn terminal_writechar(c: u8, color: u8) {
    SCREEN_DRIVER.with_text(|text| {
        if let Some(text) = text {
            text.write_char_color(c, color.into());
        } else {
            serial_println!("Terminal write char not supported in this mode");
        }
    });
}

pub fn clear_screen() {
    SCREEN_DRIVER.with_text(|text| {
        if let Some(text) = text {
            text.clear(ScreenChar::new(
                b' ',
                ColorCode::new(Color::White, Color::Black),
            ));
        }
    });
}

pub fn set_color(background: Color, foreground: Color) {
    SCREEN_DRIVER.with_text(|text| {
        if let Some(text) = text {
            let color = ColorCode::new(foreground, background);
            text.set_color(color);
        }
    });
}

pub fn disable_cursor() {
    SCREEN_DRIVER.with_text(|text| {
        if let Some(text) = text {
            text.disable_cursor();
        }
    });
}
