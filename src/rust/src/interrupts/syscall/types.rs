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
