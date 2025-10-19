use core::ffi::c_void;

use alloc::sync::Arc;

use crate::{
    bindings::{
        self, USER_CODE_SEGMENT, USER_DATA_SEGMENT, USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
        kernel_page, paging_get_physical_address, paging_switch, task_return, user_registers,
    },
    interrupts::idt::InterruptFrame,
    kernel::KERNEL,
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
                esp: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
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
        unsafe { paging_switch(self.process.page_directory as *mut u32) };
    }

    pub fn get_stack_item(&self, index: usize) -> u32 {
        let stack_pointer = self.registers.esp as *const u32;
        self.page_task();
        let res = unsafe { *(stack_pointer.add(index)) };
        unsafe { kernel_page() };
        res
    }

    pub fn virtual_address_to_physical(&self, virtual_address: *mut c_void) -> *mut c_void {
        unsafe {
            paging_get_physical_address(self.process.page_directory as *mut u32, virtual_address)
        }
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
