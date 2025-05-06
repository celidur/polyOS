use alloc::{vec, vec::Vec};

use crate::{interrupts, kernel::KERNEL};

use super::Vga;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct BmpFileHeader {
    bf_type: u16,
    bf_size: u32,
    bf_reserved1: u16,
    bf_reserved2: u16,
    bf_off_bits: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BmpInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Bitmap {
    width: u32,
    height: u32,
    image_bytes: Vec<u8>,
    total_size: usize,
    bpp: u32,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Palette {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Bitmap {
    pub fn new(filename: &str) -> Option<Bitmap> {
        let mut file = KERNEL.vfs.read().open(filename).ok()?;

        let mut file_header_buf = [0u8; size_of::<BmpFileHeader>()];
        file.ops.read(&mut file_header_buf).ok()?;
        let file_header: BmpFileHeader =
            unsafe { core::ptr::read(file_header_buf.as_ptr() as *const _) };

        let mut info_header_buf = [0u8; size_of::<BmpInfoHeader>()];
        file.ops.read(&mut info_header_buf).ok()?;
        let info_header: BmpInfoHeader =
            unsafe { core::ptr::read(info_header_buf.as_ptr() as *const _) };

        let image_size = file.ops.stat().ok()?.size as usize;

        let mut buf = vec![0u8; image_size];
        file.ops.seek(0).ok()?;
        file.ops.read(&mut buf).ok()?;

        let image_offset = file_header.bf_off_bits as usize;
        let image_bytes = buf[image_offset..].to_vec();

        Some(Bitmap {
            width: info_header.bi_width as u32,
            height: info_header.bi_height as u32,
            image_bytes,
            total_size: image_size,
            bpp: info_header.bi_bit_count as u32,
        })
    }

    pub fn display_monochrome_bitmap(&self) {
        if self.bpp != 1 {
            return;
        }

        let width = self.width as usize;
        let height = self.height as usize;

        let pixel_data = &self.image_bytes;
        interrupts::without_interrupts(|| {
            for y in 0..height {
                for x in 0..width {
                    let byte_index = (y * width + x) / 8;
                    let bit_index = x % 8;
                    let pixel = (pixel_data[byte_index] >> (7 - bit_index)) & 1;
                    let color = if pixel != 0 { 0xFF } else { 0x00 };

                    match &mut *KERNEL.vga.write() {
                        Vga::Graphic(graphic) => {
                            graphic.set_pixel(x as u32, (height - y) as u32, color);
                        }
                        _ => {}
                    }
                }
            }
        });
    }
}
