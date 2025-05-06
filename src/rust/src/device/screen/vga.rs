use crate::device::io::{inb, outb};

use super::{
    fonts::{FONT_8X8, FONT_8X16},
    framebuffer::FrameBuffer,
    graphic_vga::{GraphicVga, PixelMode},
    modes::*,
    text_vga::TextVga,
};

const VGA_GC_INDEX: u16 = 0x3CE;
const VGA_GC_DATA: u16 = 0x3CF;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum ScreenMode {
    Text(TextMode),
    Graphic(GraphicMode),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum TextMode {
    TEXT40x25,
    TEXT40x50,
    Text80x25,
    Text80x50,
    Text90x30,
    Text90x60,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum GraphicMode {
    GRAPHIC640x480x2,
    GRAPHIC320x200x4,
    GRAPHIC640x480x16,
    GRAPHIC720x480x16,
    GRAPHIC320x200x256,
    GRAPHIC320x200x256Modex,
}

#[allow(dead_code)]
pub enum Vga<'vga> {
    Text(TextVga<'vga>),
    Graphic(GraphicVga<'vga>),
    Undefined,
}

impl<'vga> Vga<'vga> {
    pub fn new(mode: ScreenMode) -> Self {
        let mut vga = Vga::Undefined;
        vga.set_mode(mode);
        vga
    }

    pub fn set_mode(&mut self, mode: ScreenMode) {
        *self = match &mode {
            ScreenMode::Text(text_mode) => {
                let (reg, cols, rows, ht) = match text_mode {
                    TextMode::TEXT40x25 => (TEXT_40X25, 40, 25, 16),
                    TextMode::TEXT40x50 => (TEXT_40X50, 40, 50, 8),
                    TextMode::Text80x25 => (TEXT_80X25, 80, 25, 8),
                    TextMode::Text80x50 => (TEXT_80X50, 80, 50, 8),
                    TextMode::Text90x30 => (TEXT_90X30, 90, 30, 8),
                    TextMode::Text90x60 => (TEXT_90X60, 90, 60, 8),
                };
                reg.write_registers();
                let reg2 = VgaRegister::read_registers();
                reg2.verify_registers(&reg);

                self.write_font(ht);

                Vga::Text(unsafe { TextVga::new(self.get_base(), cols, rows) })
            }
            ScreenMode::Graphic(graphic_mode) => {
                let (reg, wd, ht, pixel_mode) = match graphic_mode {
                    GraphicMode::GRAPHIC640x480x2 => {
                        (GRAPHIC_640X480X2, 640, 480, PixelMode::Pixel1)
                    }
                    GraphicMode::GRAPHIC320x200x4 => {
                        (GRAPHIC_320X200X4, 320, 200, PixelMode::Pixel2)
                    }
                    GraphicMode::GRAPHIC640x480x16 => {
                        (GRAPHIC_640X480X16, 640, 480, PixelMode::Pixel4p)
                    }
                    GraphicMode::GRAPHIC720x480x16 => {
                        (GRAPHIC_720X480X16, 720, 480, PixelMode::Pixel4p)
                    }
                    GraphicMode::GRAPHIC320x200x256 => {
                        (GRAPHIC_320X200X256, 320, 200, PixelMode::Pixel8)
                    }
                    GraphicMode::GRAPHIC320x200x256Modex => {
                        (GRAPHIC_320X200X256_MODEX, 320, 200, PixelMode::Pixel8x)
                    }
                };
                reg.write_registers();
                let reg2 = VgaRegister::read_registers();
                reg2.verify_registers(&reg);

                Vga::Graphic(unsafe { GraphicVga::new(self.get_base(), wd, ht, pixel_mode) })
            }
        };
    }

    fn get_base(&self) -> u32 {
        unsafe { outb(VGA_GC_INDEX, 6) };
        let seg = (unsafe { inb(VGA_GC_DATA) } >> 2) & 3;
        match seg {
            0 | 1 => 0xA0000,
            2 => 0xB0000,
            _ => 0xB8000,
        }
    }

    fn write_font(&self, font_height: usize) {
        let buf = if font_height == 8 {
            FONT_8X8.as_slice()
        } else if font_height == 16 {
            FONT_8X16.as_slice()
        } else {
            panic!("Unsupported font height");
        };

        unsafe {
            /* save registers
            set_plane() modifies GC 4 and SEQ 2, so save them as well */
            outb(VGA_SEQ_INDEX, 2);
            let seq2 = inb(VGA_SEQ_DATA);

            outb(VGA_SEQ_INDEX, 4);
            let seq4 = inb(VGA_SEQ_DATA);
            /* turn off even-odd addressing (set flat addressing)
            assume: chain-4 addressing already off */
            outb(VGA_SEQ_DATA, seq4 | 0x04);

            outb(VGA_GC_INDEX, 4);
            let gc4 = inb(VGA_GC_DATA);

            outb(VGA_GC_INDEX, 5);
            let gc5 = inb(VGA_GC_DATA);
            /* turn off even-odd addressing */
            outb(VGA_GC_DATA, gc5 & !0x10);

            outb(VGA_GC_INDEX, 6);
            let gc6 = inb(VGA_GC_DATA);
            /* turn off even-odd addressing */
            outb(VGA_GC_DATA, gc6 & !0x02);
            /* write font to plane P4 */
            set_plane(2);
            /* write font 0 */

            let base: u32 = self.get_base();
            let mut framebuffer = FrameBuffer::from_raw_parts_mut(base as *mut u8, 8192);

            for i in 0..256 {
                framebuffer.as_mut_slice()[i * 32..i * 32 + font_height]
                    .copy_from_slice(&buf[i * font_height..i * font_height + font_height]);
            }
            /* restore registers */
            outb(VGA_SEQ_INDEX, 2);
            outb(VGA_SEQ_DATA, seq2);
            outb(VGA_SEQ_INDEX, 4);
            outb(VGA_SEQ_DATA, seq4);
            outb(VGA_GC_INDEX, 4);
            outb(VGA_GC_DATA, gc4);
            outb(VGA_GC_INDEX, 5);
            outb(VGA_GC_DATA, gc5);
            outb(VGA_GC_INDEX, 6);
            outb(VGA_GC_DATA, gc6);
        }
    }

    pub fn get_text_vga(&mut self) -> Option<&mut TextVga<'vga>> {
        if let Vga::Text(vga) = self {
            Some(vga)
        } else {
            None
        }
    }

    pub fn get_graphic_vga(&mut self) -> Option<&mut GraphicVga<'vga>> {
        if let Vga::Graphic(vga) = self {
            Some(vga)
        } else {
            None
        }
    }
}

pub(super) fn set_plane(mut p: u8) {
    unsafe {
        p &= 3;
        let pmask = 1 << p;
        /* set read plane */
        outb(VGA_GC_INDEX, 4);
        outb(VGA_GC_DATA, p);
        /* set write plane */
        outb(VGA_SEQ_INDEX, 2);
        outb(VGA_SEQ_DATA, pmask);
    }
}
