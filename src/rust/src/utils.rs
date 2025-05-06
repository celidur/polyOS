use core::arch::asm;

use crate::{
    device::screen::{Bitmap, GraphicMode, ScreenMode, TextMode},
    kernel::KERNEL,
    serial_print, serial_println,
};

#[unsafe(no_mangle)]
pub extern "C" fn sync() {
    KERNEL.sync();
}

pub fn boot_image() {
    KERNEL.set_mode(ScreenMode::GRAPHIC(GraphicMode::GRAPHIC640x480x2));

    let b = Bitmap::new("/load.bmp");
    if let Some(bitmap) = b {
        bitmap.display_monochrome_bitmap();
    } else {
        serial_println!("Failed to load bitmap");
    }

    for _ in 0..100 {
        serial_print!(".");
        for _ in 0..4000000 {
            unsafe {
                asm!("nop");
            };
        }
    }
    serial_println!("Done");

    KERNEL.set_mode(ScreenMode::TEXT(TextMode::Text90x60));
}
