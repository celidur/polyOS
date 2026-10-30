use alloc::sync::Arc;
use core::arch::{asm, naked_asm};

use crate::{
    constant::{USER_CODE_SEGMENT, USER_DATA_SEGMENT},
    interrupts::{InterruptFrame, enable_interrupts},
    kernel::KERNEL,
    utils::halt,
};

use super::process::Process;

pub type TaskId = usize;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitReason {
    Io,
    Signal,
    Child(u32),
    Futex { pid: u32, address: u32 },
    Process(usize),
    PipeRead(usize),
    PipeWrite(usize),
    Semaphore(usize),
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Runnable,
    Sleeping { wake_tick: u64 },
    Blocked { reason: WaitReason },
    BlockedUntil { reason: WaitReason, wake_tick: u64 },
}

impl TaskState {
    pub fn is_runnable(self) -> bool {
        matches!(self, Self::Runnable)
    }

    pub fn is_waiting(self) -> bool {
        matches!(
            self,
            Self::Sleeping { .. } | Self::Blocked { .. } | Self::BlockedUntil { .. }
        )
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
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

pub struct Task {
    pub id: TaskId,
    pub registers: Registers,
    pub user_ticks: u64,
    pub fpu: crate::arch::FpuState,
    pub process: Arc<Process>,
    pub priority: usize,
    #[allow(dead_code)]
    pub time_slice: u32,
    pub state: TaskState,
    pub clear_child_tid: u32,
    pub signals: super::signal::ThreadSignals,
    pub name: [u8; 16],
    pub stopped: bool,
    pub restarting: bool,
    pub sleep_remaining: Option<(u32, u64)>,
    pub poll_deadline: Option<u64>,
}

impl Task {
    pub fn new(id: TaskId, process: Arc<Process>, priority: usize) -> Self {
        Self {
            id,
            registers: Self::entry_registers(&process),
            user_ticks: 0,
            fpu: Default::default(),
            name: task_name(&process),
            process,
            priority,
            time_slice: 0,
            state: TaskState::Runnable,
            clear_child_tid: 0,
            signals: Default::default(),
            stopped: false,
            restarting: false,
            sleep_remaining: None,
            poll_deadline: None,
        }
    }

    pub fn from_registers(
        id: TaskId,
        process: Arc<Process>,
        priority: usize,
        registers: Registers,
    ) -> Self {
        Self {
            id,
            registers,
            user_ticks: 0,
            fpu: Default::default(),
            name: task_name(&process),
            process,
            priority,
            time_slice: 0,
            state: TaskState::Runnable,
            clear_child_tid: 0,
            signals: Default::default(),
            stopped: false,
            restarting: false,
            sleep_remaining: None,
            poll_deadline: None,
        }
    }

    pub fn entry_registers(process: &Process) -> Registers {
        Registers {
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
            esp: process.start_stack as u32,
            ss: USER_DATA_SEGMENT,
        }
    }

    pub fn is_runnable(&self) -> bool {
        self.state.is_runnable() && !self.stopped
    }

    pub fn is_waiting(&self) -> bool {
        self.state.is_waiting() || self.stopped
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

    #[allow(dead_code)]
    pub fn virtual_address_to_physical(&self, virtual_address: u32) -> Option<u32> {
        self.process
            .page_directory
            .get_physical_address(virtual_address)
            .ok()
    }
}

enum TaskSwitch {
    Run(Registers),
    Idle,
    NoTasks,
}

pub fn task_next() -> ! {
    crate::metrics::leave();
    loop {
        let next_task = KERNEL.with_task_manager(|tm| {
            use super::task_manager::ScheduleOutcome;

            match tm.schedule() {
                ScheduleOutcome::Switched => {
                    let Some(current_task) = tm.get_current() else {
                        return TaskSwitch::NoTasks;
                    };
                    let task = current_task.read();
                    TaskSwitch::Run(task.registers)
                }
                ScheduleOutcome::Idle => TaskSwitch::Idle,
                ScheduleOutcome::NoTasks => TaskSwitch::NoTasks,
            }
        });

        match next_task {
            TaskSwitch::Run(_registers) => {
                KERNEL.kernel_page();
                super::signal::prepare_current();
                let context = KERNEL.with_task_manager(|tm| {
                    tm.get_current().and_then(|t| {
                        let t = t.read();
                        // Signal preparation can stop the selected process.
                        t.is_runnable().then_some((t.registers, t.fpu))
                    })
                });
                if let Some((registers, fpu)) = context {
                    task_page();
                    unsafe { task_return(&registers, &fpu) }
                }
            }
            TaskSwitch::Idle => {
                // No runnable task yet: wait for an interrupt to wake a sleeper.
                enable_interrupts();
                halt();
            }
            TaskSwitch::NoTasks => {
                panic!("scheduler stopped: no runnable tasks and no sleeping/blocked tasks")
            }
        }
    }
}

pub fn task_page() {
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

pub fn task_current_set_return_value(value: u32) {
    KERNEL.with_task_manager(|tm| {
        let Some(current_task) = tm.get_current() else {
            return;
        };

        current_task.write().registers.eax = value;
    });
}

pub fn user_registers() {
    unsafe {
        asm!(
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            in("ax") 0x23u16,
            options(nostack, preserves_flags)
        );
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn task_return(regs: &Registers, fpu: &crate::arch::FpuState) -> ! {
    naked_asm!(
        // Get &Registers from the stack (cdecl: at [esp+4])
        "mov  ebp, esp",
        "mov  eax, [ebp + 8]",
        "fxrstor [eax]",
        "mov  ebx, [ebp + 4]",
        // Stack frame for iret: SS, ESP, EFLAGS, CS, EIP
        "push dword ptr [ebx + 44]", // SS
        "push dword ptr [ebx + 40]", // ESP
        "mov  eax, [ebx + 36]",      // EFLAGS
        "or   eax, 0x200",           // ensure IF=1
        "push eax",
        "push dword ptr [ebx + 32]", // CS
        "push dword ptr [ebx + 28]", // EIP
        // Load data segments from SS (your code uses [ebx+44])
        "mov  ax, [ebx + 44]",
        "mov  ds, ax",
        "mov  es, ax",
        "mov  fs, ax",
        "mov  gs, ax",
        "mov  edi, [ebx + 0]",
        "mov  esi, [ebx + 4]",
        "mov  ebp, [ebx + 8]", // after this, avoid using EBP
        "mov  edx, [ebx + 16]",
        "mov  ecx, [ebx + 20]",
        "mov  eax, [ebx + 24]",
        "mov  ebx, [ebx + 12]", // restore EBX last
        "sti",
        "iretd", // 32-bit: assembles to IRETD
    );
}

fn task_name(process: &Process) -> [u8; 16] {
    let mut name = [0; 16];
    let text = process
        .executable_path
        .rsplit('/')
        .next()
        .unwrap_or("")
        .as_bytes();
    let n = text.len().min(15);
    name[..n].copy_from_slice(&text[..n]);
    name
}
