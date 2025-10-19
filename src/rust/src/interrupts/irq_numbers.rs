#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptErrorNumber {
    DoubleFault = 8,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtection = 13,
    PageFault = 14,
}

impl InterruptErrorNumber {
    pub const fn index(self) -> usize {
        self as u8 as usize
    }
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            8 => Some(Self::DoubleFault),
            10 => Some(Self::InvalidTss),
            11 => Some(Self::SegmentNotPresent),
            12 => Some(Self::StackSegmentFault),
            13 => Some(Self::GeneralProtection),
            14 => Some(Self::PageFault),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptNumber(pub u16);

impl InterruptNumber {
    pub const fn new(v: u16) -> Self {
        Self(v)
    }
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}
