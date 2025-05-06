use crate::device::io::outb;

use super::framebuffer::FrameBuffer;

const VGA_CTRL_REGISTER: u16 = 0x3d4;
const VGA_DATA_REGISTER: u16 = 0x3d5;
const VGA_OFFSET_LOW: u8 = 0x0f;
const VGA_OFFSET_HIGH: u8 = 0x0e;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

impl From<u8> for Color {
    fn from(code: u8) -> Self {
        match code {
            0 => Color::Black,
            1 => Color::Blue,
            2 => Color::Green,
            3 => Color::Cyan,
            4 => Color::Red,
            5 => Color::Magenta,
            6 => Color::Brown,
            7 => Color::LightGray,
            8 => Color::DarkGray,
            9 => Color::LightBlue,
            10 => Color::LightGreen,
            11 => Color::LightCyan,
            12 => Color::LightRed,
            13 => Color::Pink,
            14 => Color::Yellow,
            _ => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

impl From<u8> for ColorCode {
    fn from(code: u8) -> Self {
        ColorCode(code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    pub fn new(ascii_character: u8, color_code: ColorCode) -> Self {
        ScreenChar {
            ascii_character,
            color_code,
        }
    }
}

pub struct TextVga<'a> {
    ptr: FrameBuffer<'a, ScreenChar>,
    cols: usize,
    rows: usize,
    current_color: ColorCode,
    pos: (usize, usize),
}

impl TextVga<'_> {
    pub(super) unsafe fn new(base: u32, cols: usize, rows: usize) -> Self {
        let mut vga = TextVga {
            ptr: unsafe { FrameBuffer::from_raw_parts_mut(base as *mut ScreenChar, cols * rows) },
            cols,
            rows,
            current_color: ColorCode::new(Color::White, Color::Black),
            pos: (0, 0),
        };
        vga.clear(ScreenChar {
            ascii_character: b' ',
            color_code: vga.current_color,
        });
        vga
    }

    fn screen_pos(&self) -> usize {
        self.pos.0 * self.cols + self.pos.1
    }

    fn scroll_up(&mut self) {
        for i in 1..(self.rows - 1) {
            let src = i * self.cols;
            let dst = src - self.cols;
            self.ptr.copy_region(src, dst, self.cols);
        }
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.current_color,
        };
        let last_line = (self.rows - 1) * self.cols;
        self.ptr.as_mut_slice()[last_line..last_line + self.cols].fill(blank);
    }

    pub fn new_line(&mut self) {
        if self.pos.0 < self.rows - 1 {
            self.pos.0 += 1;
            self.pos.1 = 0;
            self.set_cursor(self.screen_pos() as u16);
            return;
        }

        self.pos.0 = self.rows - 1;
        self.pos.1 = 0;
        self.scroll_up();
        self.set_cursor(self.screen_pos() as u16);
    }

    pub fn backspace(&mut self) {
        if self.pos.1 > 0 {
            self.pos.1 -= 1;
        } else if self.pos.0 > 0 {
            self.pos.0 -= 1;
            self.pos.1 = self.cols - 1;
        }
        let index = self.screen_pos();
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.current_color,
        };
        self.ptr.as_mut_slice()[index] = blank;
        self.set_cursor(index as u16);
    }

    pub fn write_char_color(&mut self, ascii_character: u8, color_code: ColorCode) {
        if ascii_character == b'\n' {
            self.new_line();
            return;
        } else if ascii_character == 0x08 {
            self.backspace();
            return;
        }
        if self.pos.1 >= self.cols {
            self.new_line();
        }
        let index = self.screen_pos();
        let c = ScreenChar {
            ascii_character,
            color_code,
        };

        self.ptr.as_mut_slice()[index] = c;
        self.pos.1 += 1;
        self.set_cursor(self.screen_pos() as u16);
    }

    pub fn write_char(&mut self, ascii_character: u8) {
        self.write_char_color(ascii_character, self.current_color);
    }

    pub fn write_str_color(&mut self, s: &str, color_code: ColorCode) {
        for byte in s.bytes() {
            self.write_char_color(byte, color_code);
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_char(byte);
        }
    }

    pub fn clear(&mut self, blank: ScreenChar) {
        self.pos = (0, 0);
        self.set_cursor(0);
        self.ptr.as_mut_slice().fill(blank);
    }

    pub fn set_color(&mut self, color: ColorCode) {
        self.current_color = color;
    }

    pub fn set_cursor(&self, offset: u16) {
        unsafe {
            outb(VGA_CTRL_REGISTER, VGA_OFFSET_HIGH);
            outb(VGA_DATA_REGISTER, ((offset >> 8) & 0xff) as u8);
            outb(VGA_CTRL_REGISTER, VGA_OFFSET_LOW);
            outb(VGA_DATA_REGISTER, (offset & 0xff) as u8);
        }
    }

    pub fn disable_cursor(&self) {
        unsafe {
            outb(VGA_CTRL_REGISTER, 0x0A);
            outb(VGA_DATA_REGISTER, 0x20);
        }
    }
}

impl core::fmt::Write for TextVga<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}
