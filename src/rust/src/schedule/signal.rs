//! Pending signal delivery and trusted return context, owned by tasks and processes.
use super::{
    process::{SIG_DFL, SIG_IGN, SIGNAL_FRAME_MAGIC, SignalFrame},
    task::{Registers, Task},
};
use crate::{kernel::KERNEL, memory::user};
use alloc::vec::Vec;
pub const UNBLOCKABLE: u32 = (1 << 8) | (1 << 18);
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AltStack {
    pub address: u32,
    pub flags: u32,
    pub size: u32,
}
impl Default for AltStack {
    fn default() -> Self {
        Self {
            address: 0,
            flags: 2,
            size: 0,
        }
    }
}
struct Active {
    fpu: crate::arch::FpuState,
    address: u32,
    mask: u32,
    on_alt: bool,
}
#[derive(Default)]
pub struct ThreadSignals {
    pub blocked: u32,
    pub pending: u32,
    pub alt: AltStack,
    pub on_alt: bool,
    active: Vec<Active>,
}
pub fn frame(task: &mut Task, signal: u32) -> Result<bool, ()> {
    let process = task.process.clone();
    let action = process.get_signal_action(signal).ok_or(())?;
    if action.handler == SIG_IGN
        || (action.handler == SIG_DFL && matches!(signal, 17 | 18 | 23 | 28))
    {
        return Ok(false);
    }
    if action.handler == SIG_DFL {
        return Err(());
    }
    if task.signals.active.len() >= 16 {
        return Err(());
    }
    user::validate_range(&process, action.handler, 1, false)?;
    user::validate_range(&process, action.restorer, 1, false)?;
    let was_alt = task.signals.on_alt;
    let stack = if action.flags & 0x08000000 != 0 && task.signals.alt.flags & 2 == 0 && !was_alt {
        task.signals.on_alt = true;
        task.signals
            .alt
            .address
            .checked_add(task.signals.alt.size)
            .ok_or(())?
    } else {
        task.registers.esp
    };
    let size = core::mem::size_of::<SignalFrame>() as u32;
    let address = stack.checked_sub(size).ok_or(())? & !15;
    let call_sp = address.checked_sub(20).ok_or(())?;
    user::validate_range(&process, call_sp, stack - call_sp, true)?;
    let saved = SignalFrame {
        magic: SIGNAL_FRAME_MAGIC,
        registers: task.registers,
    };
    user::write_value(&process.page_directory, address, &saved)?;
    user::write_value(
        &process.page_directory,
        call_sp,
        &[action.restorer, signal, address],
    )?;
    task.signals.active.push(Active {
        fpu: task.fpu,
        address,
        mask: task.signals.blocked,
        on_alt: was_alt,
    });
    task.signals.blocked |= action.mask;
    if action.flags & 0x40000000 == 0 {
        task.signals.blocked |= 1 << (signal - 1);
    }
    task.signals.blocked &= !UNBLOCKABLE;
    if action.flags & 0x80000000 != 0 {
        process.set_signal_action(signal, Default::default());
    }
    task.registers.ip = action.handler;
    task.registers.esp = call_sp;
    task.registers.eax = signal;
    Ok(true)
}
/// Called with the kernel address space active, just before returning to userspace.
pub fn prepare_current() -> bool {
    let outcome = KERNEL.with_task_manager(|tm| {
        let Some(t) = tm.get_current() else {
            return (false, None);
        };
        let mut t = t.write();
        let process = t.process.clone();
        let mut pending = process.pending_signals.lock();
        let eligible = (*pending | t.signals.pending) & !t.signals.blocked;
        if eligible == 0 {
            return (false, None);
        }
        let signal = eligible.trailing_zeros() + 1;
        if t.signals.pending & (1 << (signal - 1)) != 0 {
            t.signals.pending &= !(1 << (signal - 1));
        } else {
            *pending &= !(1 << (signal - 1));
        }
        drop(pending);
        match frame(&mut t, signal) {
            Ok(changed) => (changed, None),
            Err(()) => (
                true,
                Some((
                    process.pid,
                    if process
                        .get_signal_action(signal)
                        .is_some_and(|a| a.handler == SIG_DFL)
                    {
                        signal
                    } else {
                        11
                    },
                )),
            ),
        }
    });
    if let Some((pid, signal)) = outcome.1 {
        KERNEL.with_process_manager(|pm| pm.terminate_signal(pid, signal));
    }
    outcome.0
}
pub fn restore(task: &mut Task, address: u32, mut registers: Registers) -> Result<(), ()> {
    let Some(active) = task.signals.active.last() else {
        return Err(());
    };
    if active.address != address {
        return Err(());
    }
    user::validate_range(&task.process, registers.ip, 1, false)?;
    user::validate_range(&task.process, registers.esp, 1, false)?;
    registers.cs = 0x1b;
    registers.ss = 0x23;
    registers.flags = (registers.flags & 0x00040cd5) | 0x202;
    let active = task.signals.active.pop().unwrap();
    task.fpu = active.fpu;
    task.signals.blocked = active.mask & !UNBLOCKABLE;
    task.signals.on_alt = active.on_alt;
    task.registers = registers;
    Ok(())
}
