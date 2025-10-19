mod file;
mod heap;
mod io;
mod misc;
mod process;

use file::*;
use heap::*;
use io::*;
use misc::*;
use process::*;

use crate::interrupts::idt::syscall_register;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallId {
    Serial = 0x00,
    Print = 0x01,
    GetKey = 0x02,
    PutChar = 0x03,
    Malloc = 0x04,
    Free = 0x05,
    ProcessLoadStart = 0x06,
    InvokeSystemCommand = 0x07,
    GetProcessArguments = 0x08,
    Exit = 0x09,
    PrintMemory = 0x0A,
    RemoveLastChar = 0x0B,
    ClearScreen = 0x0C,
    Fopen = 0x0D,
    Fread = 0x0E,
    Fwrite = 0x0F,
    Fseek = 0x10,
    Fstat = 0x11,
    Fclose = 0x12,
    Reboot = 0x13,
    Shutdown = 0x14,
}

impl SyscallId {
    pub const fn new(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::Serial),
            0x01 => Some(Self::Print),
            0x02 => Some(Self::GetKey),
            0x03 => Some(Self::PutChar),
            0x04 => Some(Self::Malloc),
            0x05 => Some(Self::Free),
            0x06 => Some(Self::ProcessLoadStart),
            0x07 => Some(Self::InvokeSystemCommand),
            0x08 => Some(Self::GetProcessArguments),
            0x09 => Some(Self::Exit),
            0x0A => Some(Self::PrintMemory),
            0x0B => Some(Self::RemoveLastChar),
            0x0C => Some(Self::ClearScreen),
            0x0D => Some(Self::Fopen),
            0x0E => Some(Self::Fread),
            0x0F => Some(Self::Fwrite),
            0x10 => Some(Self::Fseek),
            0x11 => Some(Self::Fstat),
            0x12 => Some(Self::Fclose),
            0x13 => Some(Self::Reboot),
            0x14 => Some(Self::Shutdown),
            _ => None,
        }
    }
}

pub fn int80h_register_commands() {
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
