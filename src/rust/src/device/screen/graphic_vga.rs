use super::{framebuffer::FrameBuffer, vga::set_plane};

pub enum PixelMode {
    Pixel1,
    Pixel2,
    Pixel4p,
    Pixel8,
    Pixel8x,
}

pub struct GraphicVga<'a> {
    ptr: FrameBuffer<'a>,
    width: u32,
    pixel_mode: PixelMode,
}

impl GraphicVga<'_> {
    pub(super) unsafe fn new(base: u32, width: u32, height: u32, pixel_mode: PixelMode) -> Self {
        let mut vga = GraphicVga {
            ptr: unsafe {
                FrameBuffer::from_raw_parts_mut(base as *mut u8, (width * height) as usize)
            },
            width,
            pixel_mode,
        };

        for y in 0..height {
            for x in 0..width {
                vga.set_pixel(x, y, 0);
            }
        }

        vga
    }

    fn write_pixel1(&mut self, x: u32, y: u32, c: u8) {
        let c = (c & 1) * 0xFF;
        let wd_in_bytes = self.width / 8;
        let off = wd_in_bytes * y + x / 8;
        let mask = 0x80 >> (x & 7);
        let ptr = self.ptr.as_mut_slice();
        ptr[off as usize] = (ptr[off as usize] & !mask) | (c & mask);
    }

    fn write_pixel2(&mut self, x: u32, y: u32, c: u8) {
        let c = (c & 3) * 0x55;
        let wd_in_bytes = self.width / 4;
        let off = wd_in_bytes * y + x / 4;
        let x = (x & 3) * 2;
        let mask = 0xC0 >> x;
        let ptr = self.ptr.as_mut_slice();
        ptr[off as usize] = (ptr[off as usize] & !mask) | (c & mask);
    }

    fn write_pixel4p(&mut self, x: u32, y: u32, c: u8) {
        let wd_in_bytes = self.width / 8;
        let off = wd_in_bytes * y + x / 8;
        let x = (x & 7) * 1;
        let mask = 0x80 >> x;
        let mut pmask = 1;
        for p in 0..4 {
            set_plane(p);
            if pmask & c != 0 {
                self.ptr.as_mut_slice()[off as usize] |= mask;
            } else {
                self.ptr.as_mut_slice()[off as usize] &= !mask;
            }
            pmask <<= 1;
        }
    }

    fn write_pixel8(&mut self, x: u32, y: u32, c: u8) {
        let wd_in_bytes = self.width;
        let off = wd_in_bytes * y + x;
        self.ptr.as_mut_slice()[off as usize] = c;
    }

    fn write_pixel8x(&mut self, x: u32, y: u32, c: u8) {
        let wd_in_bytes = self.width / 4;
        let off = wd_in_bytes * y + x / 4;
        set_plane((x & 3) as u8);
        self.ptr.as_mut_slice()[off as usize] = c;
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u8) {
        match self.pixel_mode {
            PixelMode::Pixel1 => self.write_pixel1(x as u32, y as u32, color),
            PixelMode::Pixel2 => self.write_pixel2(x as u32, y as u32, color),
            PixelMode::Pixel4p => self.write_pixel4p(x as u32, y as u32, color),
            PixelMode::Pixel8 => self.write_pixel8(x as u32, y as u32, color),
            PixelMode::Pixel8x => self.write_pixel8x(x as u32, y as u32, color),
        }
    }
}
