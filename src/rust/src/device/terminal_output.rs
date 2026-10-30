//! Bounded ANSI decoding for the VGA console; serial receives the same stream.
use super::{screen::SCREEN_DRIVER, serial::SERIAL_DRIVER};
use alloc::{format, vec::Vec};
use spin::Mutex;
struct Output {
    sequence: [u8; 64],
    used: usize,
    color: u8,
    bold: bool,
    inverse: bool,
    saved: (usize, usize),
}
static OUTPUT: Mutex<Output> = Mutex::new(Output {
    sequence: [0; 64],
    used: 0,
    color: 15,
    bold: false,
    inverse: false,
    saved: (0, 0),
});
impl Output {
    fn attribute(&self) -> u8 {
        if self.inverse {
            self.color.rotate_left(4)
        } else {
            self.color
        }
    }
    fn csi(&mut self, sequence: &[u8], replies: &mut Vec<u8>) -> bool {
        let final_byte = *sequence.last().unwrap();
        let private = sequence.get(2) == Some(&b'?');
        let mut values = [0u16; 8];
        let mut count = 1;
        for &byte in &sequence[if private { 3 } else { 2 }..sequence.len() - 1] {
            if byte == b';' {
                if count < values.len() {
                    count += 1;
                }
            } else if byte.is_ascii_digit() {
                values[count - 1] = values[count - 1]
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u16);
            }
        }
        if private {
            return false;
        } // bracketed-paste/cursor modes do not print VGA glyphs.
        if final_byte == b'm' {
            const COLORS: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
            for &value in &values[..count] {
                match value {
                    0 => {
                        self.color = 15;
                        self.bold = false;
                        self.inverse = false;
                    }
                    1 => {
                        self.bold = true;
                        self.color |= 8;
                    }
                    22 => {
                        self.bold = false;
                        self.color &= !8;
                    }
                    7 => self.inverse = true,
                    27 => self.inverse = false,
                    30..=37 => {
                        self.color = (self.color & 0xf0)
                            | COLORS[(value - 30) as usize]
                            | if self.bold { 8 } else { 0 }
                    }
                    39 => self.color = (self.color & 0xf0) | 15,
                    40..=47 => {
                        self.color = (self.color & 15) | (COLORS[(value - 40) as usize] << 4)
                    }
                    49 => self.color &= 15,
                    90..=97 => self.color = (self.color & 0xf0) | COLORS[(value - 90) as usize] | 8,
                    _ => {}
                }
            }
            return false;
        }
        let attribute = self.attribute();
        SCREEN_DRIVER.with_text(|text| {
            let Some(text) = text else {
                return;
            };
            let (row, column) = text.position();
            let amount = values[0].max(1) as usize;
            match final_byte {
                b'A' => text.move_to(row.saturating_sub(amount), column),
                b'B' => text.move_to(row.saturating_add(amount), column),
                b'C' => text.move_to(row, column.saturating_add(amount)),
                b'D' => text.move_to(row, column.saturating_sub(amount)),
                b'E' => text.move_to(row.saturating_add(amount), 0),
                b'F' => text.move_to(row.saturating_sub(amount), 0),
                b'G' => text.move_to(row, amount - 1),
                b'H' | b'f' => {
                    text.move_to(values[0].max(1) as usize - 1, values[1].max(1) as usize - 1)
                }
                b'K' => text.erase_line(values[0], attribute.into()),
                b'J' => text.erase_display(values[0], attribute.into()),
                b's' => self.saved = (row, column),
                b'u' => text.move_to(self.saved.0, self.saved.1),
                b'n' if values[0] == 6 => replies
                    .extend_from_slice(format!("\x1b[{};{}R", row + 1, column + 1).as_bytes()),
                _ => {}
            }
        });
        final_byte == b'n' && values[0] == 6
    }
}
pub fn emit(bytes: &[u8], post: bool) -> Vec<u8> {
    let mut output = OUTPUT.lock();
    let mut replies = Vec::new();
    for &byte in bytes {
        if output.used != 0 {
            let used = output.used;
            output.sequence[used] = byte;
            output.used += 1;
            let complete =
                (used == 1 && byte != b'[') || (used > 1 && (0x40..=0x7e).contains(&byte));
            if complete || output.used == output.sequence.len() {
                let sequence = output.sequence;
                let len = output.used;
                let reply =
                    len >= 3 && sequence[1] == b'[' && output.csi(&sequence[..len], &mut replies);
                // The console answers cursor queries itself; suppress serial
                // forwarding to avoid a second reply from an external emulator.
                if !reply {
                    SERIAL_DRIVER.write(&sequence[..len]);
                }
                output.used = 0;
            }
            continue;
        }
        if byte == 0x1b {
            output.sequence[0] = byte;
            output.used = 1;
            continue;
        }
        if post && byte == b'\n' {
            SERIAL_DRIVER.write(b"\r");
        }
        SERIAL_DRIVER.write(&[byte]);
        crate::print::terminal_writechar(byte, output.attribute());
    }
    replies
}
