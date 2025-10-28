use core::arch::naked_asm;

use crate::{constant::KERNEL_DATA_SELECTOR, kernel_main, utils::halt};

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".start")]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "mov ax, {kds}",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov ss, ax",
        "mov ebp, 0x00200000",
        "mov esp, ebp",

        "mov al, 0x11", // == 0b00010001
        "out 0x20, al", // Tell master PIC

        "mov al, 0x20", // Interrupt 0x20 is where master ISR should start
        "out 0x21, al",

        "mov al, 0x04",// ICW3
        "out 0x21, al",

        "mov al, 0x01", // == 0b00000001
        "out 0x21, al", // end remapping of the master PIC

        "call {kernel_main}",

        "call {halt}",

        kds = const KERNEL_DATA_SELECTOR,
        kernel_main = sym kernel_main,
        halt = sym halt,
    );
}
