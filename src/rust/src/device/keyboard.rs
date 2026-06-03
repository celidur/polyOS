use alloc::boxed::Box;
use alloc::collections::VecDeque;

use crate::{
    constant::irq_to_vector,
    device::{DeviceDriver, DeviceProbeStage, ManagedDevice},
    fs::{FileHandle, FileMetadata, FileOps, FsError},
    interrupts::{InterruptDevice, InterruptSource},
};

use super::io::{inb, outb};

// Constants
const KEYBOARD_IRQ_LINE: u8 = 1;
const PS2_PORT: u16 = 0x64;
const PS2_COMMAND_ENABLE_FIRST_PORT: u8 = 0xAE;
const KEYBOARD_INPUT_PORT: u16 = 0x60;
const CLASSIC_KEYBOARD_KEY_RELEASED: u8 = 0x80;

const SHIFT_LEFT: u8 = 0x2A;
const SHIFT_RIGHT: u8 = 0x36;
const SHIFT_LOCK: u8 = 0x3A;
const CTRL: u8 = 0x1D;

const ESC: u8 = 0x1B;
const BS: u8 = 0x08;
const ENTER: u8 = 0x0D;

pub struct Keyboard {
    shift: u8,
    ctrl: u8,
    input: VecDeque<u8>,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            shift: 0,
            ctrl: 0,
            input: VecDeque::new(),
        }
    }

    fn handle_interrupt(&mut self) {
        let mut scancode = unsafe { inb(KEYBOARD_INPUT_PORT) };
        unsafe { inb(KEYBOARD_INPUT_PORT) }; // Discard second read (buffered byte)

        if scancode & CLASSIC_KEYBOARD_KEY_RELEASED != 0 {
            scancode &= !CLASSIC_KEYBOARD_KEY_RELEASED;
            match scancode {
                SHIFT_LEFT => self.shift &= !0x01,
                SHIFT_RIGHT => self.shift &= !0x02,
                CTRL => self.ctrl = 0,
                _ => {}
            }
            return;
        }

        match scancode {
            SHIFT_LEFT => {
                self.shift |= 0x01;
                return;
            }
            SHIFT_RIGHT => {
                self.shift |= 0x02;
                return;
            }
            CTRL => {
                self.ctrl = 1;
                return;
            }
            SHIFT_LOCK => {
                self.shift ^= 0x04;
                return;
            }
            _ => {}
        }

        let ch = self.scancode_to_char(scancode);
        if ch != 0 {
            self.input.push_back(ch);
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        self.input.pop_front()
    }

    fn scancode_to_char(&self, scancode: u8) -> u8 {
        let map = if self.shift != 0 {
            &KEYBOARD_SCAN_SET_TWO
        } else {
            &KEYBOARD_SCAN_SET_ONE
        };

        *map.get(scancode as usize).unwrap_or(&0)
    }
}

// Scancode sets
static KEYBOARD_SCAN_SET_ONE: [u8; 92] = [
    0x00, ESC, b'1', b'2', /* 0x00 */
    b'3', b'4', b'5', b'6', /* 0x04 */
    b'7', b'8', b'9', b'0', /* 0x08 */
    b'-', b'=', BS, b'\t', /* 0x0C */
    b'q', b'w', b'e', b'r', /* 0x10 */
    b't', b'y', b'u', b'i', /* 0x14 */
    b'o', b'p', b'[', b']', /* 0x18 */
    ENTER, 0x00, b'a', b's', /* 0x1C */
    b'd', b'f', b'g', b'h', /* 0x20 */
    b'j', b'k', b'l', b';', /* 0x24 */
    b'\'', b'`', 0x00, b'\\', /* 0x28 */
    b'z', b'x', b'c', b'v', /* 0x2C */
    b'b', b'n', b'm', b',', /* 0x30 */
    b'.', b'/', 0x00, b'*', /* 0x34 */
    0x00, b' ', 0x00, 0x00, /* 0x38 */
    0x00, 0x00, 0x00, 0x00, /* 0x3C */
    0x00, 0x00, 0x00, 0x00, /* 0x40 */
    0x00, 0x00, 0x00, b'7', /* 0x44 */
    b'8', b'9', b'-', b'4', /* 0x48 */
    b'5', b'6', b'+', b'1', /* 0x4C */
    b'2', b'3', b'0', b'.', /* 0x50 */
    0x00, 0x00, 0x00, 0x00, /* 0x54 */
    0x00, 0x00, 0x00, 0x00, /* 0x58 */
];

static KEYBOARD_SCAN_SET_TWO: [u8; 92] = [
    0x00, ESC, b'!', b'@', /* 0x00 */
    b'#', b'$', b'%', b'^', /* 0x04 */
    b'&', b'*', b'(', b')', /* 0x08 */
    b'_', b'+', BS, b'\t', /* 0x0C */
    b'Q', b'W', b'E', b'R', /* 0x10 */
    b'T', b'Y', b'U', b'I', /* 0x14 */
    b'O', b'P', b'{', b'}', /* 0x18 */
    ENTER, 0x00, b'A', b'S', /* 0x1C */
    b'D', b'F', b'G', b'H', /* 0x20 */
    b'J', b'K', b'L', b':', /* 0x24 */
    b'"', b'~', 0x00, b'|', /* 0x28 */
    b'Z', b'X', b'C', b'V', /* 0x2C */
    b'B', b'N', b'M', b'<', /* 0x30 */
    b'>', b'?', 0x00, b'*', /* 0x34 */
    0x00, b' ', 0x00, 0x00, /* 0x38 */
    0x00, 0x00, 0x00, 0x00, /* 0x3C */
    0x00, 0x00, 0x00, 0x00, /* 0x40 */
    0x00, 0x00, 0x00, b'7', /* 0x44 */
    b'8', b'9', b'-', b'4', /* 0x48 */
    b'5', b'6', b'+', b'1', /* 0x4C */
    b'2', b'3', b'0', b'.', /* 0x50 */
    0x00, 0x00, 0x00, 0x00, /* 0x54 */
    0x00, 0x00, 0x00, 0x00, /* 0x58 */
];

pub struct KeyboardDriver {
    device: ManagedDevice<Keyboard>,
}

impl KeyboardDriver {
    pub const fn new() -> Self {
        Self {
            device: ManagedDevice::new(),
        }
    }

    pub fn read_byte(&self) -> Option<u8> {
        self.device
            .with_mut(|keyboard| keyboard.read_byte())
            .flatten()
    }
}

pub static KEYBOARD_DRIVER: KeyboardDriver = KeyboardDriver::new();

struct KeyboardFile;

impl FileOps for KeyboardFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        let mut read = 0;

        while read < buf.len() {
            let Some(byte) = KEYBOARD_DRIVER.read_byte().map(normalize_input) else {
                break;
            };

            buf[read] = byte;
            read += 1;

            if byte == 13 {
                break;
            }
        }

        Ok(read)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn seek(&mut self, _pos: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(FileMetadata {
            uid: 0,
            gid: 0,
            mode: 0o666,
            size: 0,
            is_dir: false,
        })
    }
}

impl DeviceDriver for KeyboardDriver {
    fn name(&self) -> &'static str {
        "keyboard"
    }

    fn stage(&self) -> DeviceProbeStage {
        DeviceProbeStage::Normal
    }

    fn probe(&self) {
        unsafe { outb(PS2_PORT, PS2_COMMAND_ENABLE_FIRST_PORT) };
        let keyboard_interrupt = irq_to_vector(KEYBOARD_IRQ_LINE)
            .expect("keyboard IRQ line must map to an interrupt vector");
        KEYBOARD_DRIVER
            .device
            .probe(Keyboard::new())
            .expect("keyboard device already probed");
        InterruptSource::new(keyboard_interrupt)
            .register_device(&KEYBOARD_DRIVER);
    }

    fn remove(&self) {
        let _ = KEYBOARD_DRIVER.device.remove();
    }
}

impl InterruptDevice for KeyboardDriver {
    fn interrupt(&self) {
        let _ = self.device.with_mut(|keyboard| keyboard.handle_interrupt());
    }
}

pub fn open_keyboard() -> FileHandle {
    FileHandle::new(Box::new(KeyboardFile))
}

fn normalize_input(byte: u8) -> u8 {
    match byte {
        b'\n' => 13,
        0x7f => 0x08,
        byte => byte,
    }
}

crate::register_device_node!(KEYBOARD_DEVICE_NODE_REG, ["keyboard", "kbd"], open_keyboard);
crate::register_device_driver!(KEYBOARD_DRIVER_REG, KEYBOARD_DRIVER);
