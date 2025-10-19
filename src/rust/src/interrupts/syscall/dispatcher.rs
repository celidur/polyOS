use crate::interrupts::InterruptFrame;

use super::file::*;
use super::heap::*;
use super::io::*;
use super::misc::*;
use super::process::*;
use super::register::{syscall_get_handler, syscall_register};
use super::types::SyscallId;

pub fn syscall_init() {
    syscall_register(SyscallId::Serial, int80h_command0_serial);
    syscall_register(SyscallId::Print, int80h_command1_print);
    syscall_register(SyscallId::GetKey, int80h_command2_getkey);
    syscall_register(SyscallId::PutChar, int80h_command3_putchar);
    syscall_register(SyscallId::Malloc, int80h_command4_malloc);
    syscall_register(SyscallId::Free, int80h_command5_free);
    syscall_register(
        SyscallId::ProcessLoadStart,
        int80h_command6_process_load_start,
    );
    syscall_register(
        SyscallId::InvokeSystemCommand,
        int80h_command7_invoke_system_command,
    );
    syscall_register(
        SyscallId::GetProcessArguments,
        int80h_command8_get_program_arguments,
    );
    syscall_register(SyscallId::Exit, int80h_command9_exit);
    syscall_register(SyscallId::PrintMemory, int80h_command10_print_memory);
    syscall_register(SyscallId::RemoveLastChar, int80h_command11_remove_last_char);
    syscall_register(SyscallId::ClearScreen, int80h_command12_clear_screen);
    syscall_register(SyscallId::Fopen, int80h_command13_fopen);
    syscall_register(SyscallId::Fread, int80h_command14_fread);
    syscall_register(SyscallId::Fwrite, int80h_command15_fwrite);
    syscall_register(SyscallId::Fseek, int80h_command16_fseek);
    syscall_register(SyscallId::Fstat, int80h_command17_fstat);
    syscall_register(SyscallId::Fclose, int80h_command18_fclose);
    syscall_register(SyscallId::Reboot, int80h_command19_reboot);
    syscall_register(SyscallId::Shutdown, int80h_command20_shutdown);
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
