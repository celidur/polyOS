use core::arch::naked_asm;

use crate::{
    constant::{
        KERNEL_DATA_SELECTOR, PIC_MASTER_COMMAND_PORT, PIC_MASTER_DATA_PORT,
        PIC_MASTER_VECTOR_OFFSET, PIC_SLAVE_COMMAND_PORT, PIC_SLAVE_DATA_PORT, PIC_SLAVE_IRQ_LINE,
        PIC_SLAVE_IRQ_MASK, PIC_SLAVE_VECTOR_OFFSET,
    },
    kernel_main,
    memory::init_heap,
    utils::halt_forever,
};

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
        "out {pic_master_cmd}, al", // Tell master PIC
        "out {pic_slave_cmd}, al", // Tell slave PIC

        "mov al, {pic_master_vector}", // Interrupt base for master IRQs
        "out {pic_master_data}, al",

        "mov al, {pic_slave_vector}", // Interrupt base for slave IRQs
        "out {pic_slave_data}, al",

        "mov al, {pic_slave_irq_mask}", // Tell master about the slave on IRQ2
        "out {pic_master_data}, al",

        "mov al, {pic_slave_irq_line}", // Slave PIC is connected to master's IRQ2
        "out {pic_slave_data}, al",

        "mov al, 0x01", // == 0b00000001
        "out {pic_master_data}, al", // end remapping of the master PIC
        "out {pic_slave_data}, al", // end remapping of the slave PIC

        // init heap
        "call {init_heap}",

        "call {kernel_main}",

        "call {halt_forever}",

        kds = const KERNEL_DATA_SELECTOR,
        pic_master_cmd = const PIC_MASTER_COMMAND_PORT,
        pic_slave_cmd = const PIC_SLAVE_COMMAND_PORT,
        pic_master_data = const PIC_MASTER_DATA_PORT,
        pic_slave_data = const PIC_SLAVE_DATA_PORT,
        pic_master_vector = const PIC_MASTER_VECTOR_OFFSET,
        pic_slave_vector = const PIC_SLAVE_VECTOR_OFFSET,
        pic_slave_irq_line = const PIC_SLAVE_IRQ_LINE,
        pic_slave_irq_mask = const PIC_SLAVE_IRQ_MASK,
        init_heap = sym init_heap,
        kernel_main = sym kernel_main,
        halt_forever = sym halt_forever,
    );
}
