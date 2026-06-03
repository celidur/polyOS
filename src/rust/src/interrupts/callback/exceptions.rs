use crate::{
    interrupts::{interrupt_frame::InterruptFrame, utils::get_cr2},
    schedule::{process_manager::process_terminate, task::task_next},
};

pub fn idt_handle_exception(_frame: &InterruptFrame) {
    process_terminate(1);
    task_next();
}

pub fn idt_handle_exception_error(_frame: &InterruptFrame, _error_code: u32) {
    process_terminate(1);
    task_next();
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

    if p != 0 && w != 0 {
        let handled = crate::kernel::KERNEL
            .with_task_manager(|tm| tm.get_current().map(|t| t.read().process.clone()))
            .and_then(|process| process.handle_cow_fault(faulting_address).ok())
            .unwrap_or(false);

        if handled {
            return;
        }
    }

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

    let (ip, esp, eax, ebx, ecx, edx, esi, edi, ebp) = (
        frame.ip, frame.esp, frame.eax, frame.ebx, frame.ecx, frame.edx, frame.esi, frame.edi,
        frame.ebp,
    );
    serial_println!("EIP: 0x{:x}  ESP: 0x{:x}", ip, esp);
    serial_println!(
        "EAX: 0x{:x}  EBX: 0x{:x}  ECX: 0x{:x}  EDX: 0x{:x}",
        eax,
        ebx,
        ecx,
        edx
    );
    serial_println!("ESI: 0x{:x}  EDI: 0x{:x}  EBP: 0x{:x}", esi, edi, ebp);

    let pid = crate::kernel::KERNEL
        .with_task_manager(|tm| tm.get_current().map(|t| t.read().process.pid));
    serial_println!("Faulting process PID: {:?}", pid);

    process_terminate(1);
    task_next();
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
