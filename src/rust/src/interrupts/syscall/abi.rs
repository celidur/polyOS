pub const ERROR: u32 = (-1_i32) as u32;
pub const WAIT_TIMEOUT: u32 = (-2_i32) as u32;

#[inline]
pub const fn error() -> u32 {
    ERROR
}
