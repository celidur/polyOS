pub const TIOCGWINSZ: u32 = 0x5413;
pub const POLYOS_IOCTL_SCREEN_CLEAR: u32 = 0x5001;
pub const POLYOS_IOCTL_SCREEN_SET_COLOR: u32 = 0x5002;
pub const POLYOS_IOCTL_SCREEN_DISABLE_CURSOR: u32 = 0x5003;

#[repr(C)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}
