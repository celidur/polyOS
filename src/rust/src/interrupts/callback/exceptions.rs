use crate::{
    interrupts::{interrupt_frame::InterruptFrame, utils::get_cr2},
    schedule::{process::process_terminate, task::task_next},
};

pub fn idt_handle_exception(_frame: &InterruptFrame) {
    process_terminate();
    task_next();
    panic!("No more tasks to run\n");
}

pub fn idt_handle_exception_error(_frame: &InterruptFrame, _error_code: u32) {
    process_terminate();
    task_next();
    panic!("No more tasks to run\n");
}

pub fn idt_page_fault(frame: &InterruptFrame, code_error: u32) {
    let faulting_address = get_cr2();

    let p = code_error & 0x1;
    let w = (code_error >> 1) & 0x1;
    let u = (code_error >> 2) & 0x1;
    let r = (code_error >> 3) & 0x1;
    let i = (code_error >> 4) & 0x1;
    let pk = (code_error >> 5) & 0x1;
    let ss = (code_error >> 6) & 0x1;
    let sgx = (code_error >> 15) & 0x1;

    serial_print!("Page fault( ");
    if p != 0 {
        serial_print!("protection violation ");
    }
    if w != 0 {
        serial_print!("write ");
    } else {
        serial_print!("read ");
    }
    if u != 0 {
        serial_print!("user ");
    } else {
        serial_print!("supervisor ");
    }
    if r != 0 {
        serial_print!("reserved ");
    }
    if i != 0 {
        serial_print!("instruction fetch ");
    }
    if pk != 0 {
        serial_print!("protection key violation ");
    }
    if ss != 0 {
        serial_print!("shadow stack ");
    }
    if sgx != 0 {
        serial_print!("SGX ");
    }
    serial_println!(") at 0x{:x}", faulting_address);

    serial_println!("Register:");
    serial_println!("{:?}", frame);

    panic!("Page fault");
}

pub fn idt_general_protection_fault(_frame: &InterruptFrame, code_error: u32) {
    serial_println!("{:?}", _frame);
    serial_println!("General protection fault");
    let e = code_error & 0x1;
    if e != 0 {
        serial_println!("the exception originated externally to the processor");
    } else {
        let tbl = (code_error >> 1) & 0x3;
        let index = (code_error >> 3) & 0x1FFF;
        match tbl {
            0 => serial_print!("GDT"),
            1 | 3 => serial_print!("IDT"),
            2 => serial_print!("LDT"),
            _ => {}
        }
        serial_println!(" index: 0x{:x}", index);
    }
    panic!("General protection fault");
}
