use core::arch::asm;

use crate::{
    device::{
        io::{inb, outb, outw},
        screen::{Bitmap, GraphicMode, ScreenMode, TextMode},
    },
    interrupts::disable_interrupts,
    kernel::KERNEL,
    serial_print, serial_println,
};

#[unsafe(no_mangle)]
pub extern "C" fn sync() {
    KERNEL.sync();
}

pub fn boot_image() {
    KERNEL.set_mode(ScreenMode::Graphic(GraphicMode::GRAPHIC640x480x2));

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

    KERNEL.set_mode(ScreenMode::Text(TextMode::Text90x60));
}

pub fn shutdown() {
    serial_println!("Shutting down...");

    unsafe { outw(0x604, 0x2000) };

    halt();
}

pub fn reboot() {
    let mut good = 0x02;
    disable_interrupts();
    while good & 0x02 != 0 {
        good = unsafe { inb(0x64) };
    }
    serial_println!("Rebooting...");
    unsafe { outb(0x64, 0xFE) };
    halt();
}

pub fn halt() -> ! {
    serial_println!("Halting CPU...");

    loop {
        unsafe {
            asm!(
                "
        hlt
        ",
                options(nostack, nomem)
            )
        }
    }
}
