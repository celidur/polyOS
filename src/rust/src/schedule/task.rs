use core::arch::asm;

use alloc::sync::Arc;

use crate::{
    bindings::{self, task_return, user_registers},
    constant::{
        PAGING_PAGE_SIZE, USER_CODE_SEGMENT, USER_DATA_SEGMENT,
        USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
    },
    interrupts::InterruptFrame,
    kernel::KERNEL,
    memory::{self, PageDirectory},
};

use super::process::Process;

pub type TaskId = usize;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Registers {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
    pub esp: u32,
    pub ss: u32,
}

pub enum TaskState {
    Runnable,
    // Waiting,
    // Terminated,
}

pub struct Task {
    pub id: TaskId,
    pub registers: Registers,
    pub state: TaskState,
    pub process: Arc<Process>,
    pub priority: usize,
    pub time_slice: u32,
}

impl Task {
    pub fn new(id: TaskId, process: Arc<Process>, priority: usize) -> Self {
        Self {
            id,
            registers: Registers {
                edi: 0,
                esi: 0,
                ebp: 0,
                ebx: 0,
                edx: 0,
                ecx: 0,
                eax: 0,
                ip: process.entrypoint,
                cs: USER_CODE_SEGMENT,
                flags: 0,
                esp: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START as u32,
                ss: USER_DATA_SEGMENT,
            },
            state: TaskState::Runnable,
            process,
            priority,
            time_slice: 0,
        }
    }

    pub fn set_state(&mut self, state: &InterruptFrame) {
        self.registers.edi = state.edi;
        self.registers.esi = state.esi;
        self.registers.ebp = state.ebp;
        self.registers.ebx = state.ebx;
        self.registers.edx = state.edx;
        self.registers.ecx = state.ecx;
        self.registers.eax = state.eax;
        self.registers.ip = state.ip;
        self.registers.cs = state.cs;
        self.registers.flags = state.flags;
        self.registers.esp = state.esp;
        self.registers.ss = state.ss;
    }

    pub fn page_task(&self) {
        unsafe { user_registers() };
        self.process.page_directory.switch();
    }

    pub fn get_stack_item(&self, index: usize) -> u32 {
        let stack_pointer = self.registers.esp as *const u32;
        self.page_task();
        let res = unsafe { *(stack_pointer.add(index)) };
        KERNEL.kernel_page();
        res
    }

    pub fn virtual_address_to_physical(&self, virtual_address: u32) -> Option<u32> {
        self.process
            .page_directory
            .get_physical_address(virtual_address)
            .ok()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn task_next() {
    let registers = KERNEL.with_task_manager(|tm| {
        let _ = tm.schedule();
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some(task.registers)
    });

    if let Some(registers) = registers {
        unsafe { task_return((&registers) as *const _ as *mut _) };
    }
    panic!("Failed to return to task");
}

#[unsafe(no_mangle)]
pub extern "C" fn task_page() {
    KERNEL.with_task_manager(|tm| {
        let _ = tm.task_page();
    });
}

pub fn task_current_save_state(frame: &InterruptFrame) {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return;
        };
        let mut task = current_task.write();
        task.set_state(frame);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn get_register() -> *mut bindings::registers {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return core::ptr::null_mut();
        };
        let task = current_task.read();
        &task.registers as *const Registers as *mut bindings::registers
    })
}

pub fn copy_string_from_task(
    directory: &PageDirectory,
    virt: u32,
    phys: u32,
    size: u32,
) -> Result<(), ()> {
    let mut remain = size;
    let flags = memory::PRESENT | memory::USER_ACCESS | memory::WRITABLE;

    let mut virt = virt;
    let mut phys = phys;

    while remain > 0 {
        let to_copy = remain.min(PAGING_PAGE_SIZE as u32);
        let mut page = memory::Page::new(to_copy as usize).ok_or(())?;
        let page_addr = page.as_ptr() as u32;
        let old_entry = directory.get(page_addr).map_err(|_| ())?;
        directory
            .map_page(page_addr, &page, flags)
            .map_err(|_| ())?;
        let buffer = unsafe { core::slice::from_raw_parts_mut(virt as *mut u8, to_copy as usize) };
        let buffer2 = page.as_mut_slice();
        directory.switch();
        buffer2[..to_copy as usize].copy_from_slice(&buffer[..to_copy as usize]);
        KERNEL.kernel_page();
        directory.set(page_addr, old_entry).map_err(|_| ())?;
        remain -= to_copy;
        let buffer = unsafe { core::slice::from_raw_parts_mut(phys as *mut u8, to_copy as usize) };
        buffer[..to_copy as usize].copy_from_slice(&buffer2[..to_copy as usize]);
        virt += to_copy;
        phys += to_copy;
    }

    Ok(())
}

pub fn copy_string_to_task(
    directory: &PageDirectory,
    buff: u32,
    virt: u32,
    size: u32,
) -> Result<(), ()> {
    let mut remain = size;
    let flags = memory::PRESENT | memory::USER_ACCESS | memory::WRITABLE;

    let mut virt = virt;
    let mut buff = buff;

    while remain > 0 {
        let phs_addr = PageDirectory::align_address_down(buff);
        let old_entry = directory.get(phs_addr).map_err(|_| ())?;
        directory
            .map_page(
                phs_addr,
                &memory::Page::new(PAGING_PAGE_SIZE as usize).ok_or(())?,
                flags,
            )
            .map_err(|_| ())?;
        let offset = buff - phs_addr;
        let to_copy = (PAGING_PAGE_SIZE as u32 - offset).min(remain);
        let buffer = unsafe { core::slice::from_raw_parts_mut(virt as *mut u8, to_copy as usize) };
        let buffer2 = unsafe {
            core::slice::from_raw_parts_mut((phs_addr + offset) as *mut u8, to_copy as usize)
        };
        directory.switch();
        buffer2[..to_copy as usize].copy_from_slice(&buffer[..to_copy as usize]);
        KERNEL.kernel_page();
        directory.set(phs_addr, old_entry).map_err(|_| ())?;
        remain -= to_copy;
        virt += to_copy;
        buff += to_copy;
    }

    Ok(())
}

// pub fn user_registers() {
//     unsafe {
//         asm!(
//             "mov ax, 0x23",
//             "mov ds, ax",
//             "mov es, ax",
//             "mov fs, ax",
//             "mov gs, ax",
//             "ret",
//             options(nostack)
//         );
//     }
// }

// pub fn restore_general_registers(ctx: *const Registers) {
//     // cdecl: [esp+4] holds `ctx`
//     unsafe {
//         asm!(
//             "push ebp",
//             "mov  ebp, esp",
//             "mov  ebx, [ebp + 8]", // ebx = ctx
//             "mov  edi, [ebx + 0]", // edi
//             "mov  esi, [ebx + 4]", // esi
//             "mov  ebp, [ebx + 8]", // ebp
//             "mov  edx, [ebx + 16]", // edx
//             "mov  ecx, [ebx + 20]", // ecx
//             "mov  eax, [ebx + 24]", // eax
//             "mov  ebx, [ebx + 12]", // ebx (last since we used ebx as ctx)
//             "add  esp, 4",         // pop arg
//             "ret",
//             options(noreturn)
//         );
//     }
// }
