use crate::interrupts::InterruptFrame;

use super::file::*;
use super::heap::*;
use super::io::*;
use super::misc::*;
use super::process::*;
use super::register::{syscall_get_handler, syscall_register};
use super::types::SyscallId;

pub fn syscall_init() {
    syscall_register(SyscallId::Serial, syscall_serial);
    syscall_register(SyscallId::Print, syscall_print);
    syscall_register(SyscallId::GetKey, syscall_getkey);
    syscall_register(SyscallId::PutChar, syscall_putchar);
    syscall_register(SyscallId::Malloc, syscall_malloc);
    syscall_register(SyscallId::Free, syscall_free);
    syscall_register(SyscallId::ProcessLoadStart, syscall_process_load_start);
    syscall_register(SyscallId::Exec, syscall_exec);
    syscall_register(
        SyscallId::GetProcessArguments,
        syscall_get_program_arguments,
    );
    syscall_register(SyscallId::Exit, syscall_exit);
    syscall_register(SyscallId::PrintMemory, syscall_print_memory);
    syscall_register(SyscallId::RemoveLastChar, syscall_remove_last_char);
    syscall_register(SyscallId::ClearScreen, syscall_clear_screen);
    syscall_register(SyscallId::Fopen, syscall_fopen);
    syscall_register(SyscallId::Fread, syscall_fread);
    syscall_register(SyscallId::Fwrite, syscall_fwrite);
    syscall_register(SyscallId::Fseek, syscall_fseek);
    syscall_register(SyscallId::Fstat, syscall_fstat);
    syscall_register(SyscallId::Fclose, syscall_fclose);
    syscall_register(SyscallId::Reboot, syscall_reboot);
    syscall_register(SyscallId::Shutdown, syscall_shutdown);
}

pub fn syscall_handle(frame: &InterruptFrame) -> u32 {
    let cmd = frame.eax;
    let cmd = match SyscallId::new(cmd as u8) {
        Some(c) => c,
        None => {
            serial_println!("Unknown syscall command: {}", cmd);
            return u32::MAX;
        }
    };

    syscall_get_handler(cmd)
        .map(|handler| handler(frame))
        .unwrap_or_else(|| {
            serial_println!("Unknown syscall command: {:?}", cmd);
            u32::MAX
        })
}
