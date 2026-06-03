use alloc::boxed::Box;

use crate::{
    device::{control::*, DeviceDriver, DeviceProbeStage, ManagedDevice},
    fs::{FileHandle, FileMetadata, FileOps, FsError},
    memory::PageDirectory,
};

use super::{
    Color, ColorCode, ScreenChar, ScreenMode, TextMode, TextVga, Vga,
};

pub struct ScreenDriver {
    device: ManagedDevice<Vga<'static>>,
}

impl ScreenDriver {
    pub const fn new() -> Self {
        Self {
            device: ManagedDevice::new(),
        }
    }

    pub fn set_mode(&self, mode: ScreenMode) {
        self.device
            .with_mut(|vga| vga.set_mode(mode))
            .expect("screen device not probed");
    }

    pub fn with_text<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Option<&mut TextVga<'_>>) -> R,
    {
        let mut f = Some(f);
        match self.device.with_mut(|vga| f.take().unwrap()(vga.get_text_vga())) {
            Some(result) => result,
            None => f.take().unwrap()(None),
        }
    }

    pub fn with_graphic<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Option<&mut super::GraphicVga<'_>>) -> R,
    {
        let mut f = Some(f);
        match self.device.with_mut(|vga| f.take().unwrap()(vga.get_graphic_vga())) {
            Some(result) => result,
            None => f.take().unwrap()(None),
        }
    }

    pub fn clear(&self) {
        self.with_text(|text| {
            if let Some(text) = text {
                text.clear(ScreenChar::new(
                    b' ',
                    ColorCode::new(Color::White, Color::Black),
                ));
            }
        });
    }

    pub fn disable_cursor(&self) {
        self.with_text(|text| {
            if let Some(text) = text {
                text.disable_cursor();
            }
        });
    }

    pub fn set_color(&self, color: ColorCode) {
        self.with_text(|text| {
            if let Some(text) = text {
                text.set_color(color);
            }
        });
    }
}

pub static SCREEN_DRIVER: ScreenDriver = ScreenDriver::new();

impl DeviceDriver for ScreenDriver {
    fn name(&self) -> &'static str {
        "vga"
    }

    fn stage(&self) -> DeviceProbeStage {
        DeviceProbeStage::Early
    }

    fn probe(&self) {
        let vga: Vga<'static> = Vga::new(ScreenMode::Text(TextMode::Text90x60));
        self.device
            .probe(vga)
            .expect("screen device already probed");
    }

    fn remove(&self) {
        let _ = self.device.remove();
    }
}

struct ScreenFile;

impl FileOps for ScreenFile {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        SCREEN_DRIVER.with_text(|text| {
            if let Some(text) = text {
                for byte in buf.iter().copied() {
                    text.write_char(byte);
                }
            }
        });
        Ok(buf.len())
    }

    fn seek(&mut self, _pos: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn ioctl(&mut self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        match request {
            TIOCGWINSZ => {
                if arg == 0 {
                    return Err(FsError::InvalidArgument);
                }

                let Some((rows, cols)) = SCREEN_DRIVER.with_text(|text| {
                    text.map(|text| (text.rows() as u16, text.cols() as u16))
                }) else {
                    return Err(FsError::Unsupported);
                };

                let size = WinSize {
                    ws_row: rows,
                    ws_col: cols,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };

                crate::schedule::task::copy_string_to_task(
                    directory,
                    &size as *const WinSize as u32,
                    arg,
                    core::mem::size_of::<WinSize>() as u32,
                )
                .map_err(|_| FsError::IoError)?;

                Ok(0)
            }
            POLYOS_IOCTL_SCREEN_CLEAR => {
                SCREEN_DRIVER.clear();
                Ok(0)
            }
            POLYOS_IOCTL_SCREEN_SET_COLOR => {
                SCREEN_DRIVER.set_color(ColorCode::from(arg as u8));
                Ok(0)
            }
            POLYOS_IOCTL_SCREEN_DISABLE_CURSOR => {
                SCREEN_DRIVER.disable_cursor();
                Ok(0)
            }
            _ => Err(FsError::Unsupported),
        }
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

pub fn open_vga() -> FileHandle {
    FileHandle::new(Box::new(ScreenFile))
}

crate::register_device_driver!(SCREEN_DRIVER_REG, SCREEN_DRIVER);
crate::register_device_node!(SCREEN_DEVICE_NODE_REG, ["screen", "vga"], open_vga);
