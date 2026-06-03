use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use crate::{
    device::{
        control::{
            POLYOS_IOCTL_SCREEN_CLEAR, POLYOS_IOCTL_SCREEN_DISABLE_CURSOR,
            POLYOS_IOCTL_SCREEN_SET_COLOR, TIOCGWINSZ, WinSize,
        },
        screen::ColorCode,
    },
    kernel::KERNEL,
    memory::PageDirectory,
    print::{clear_screen, disable_cursor, terminal_writechar},
    schedule::task::copy_string_to_task,
};

use super::vfs::{
    FileHandle, FileMetadata, FileOps, FileSystem, FileSystemDriver, FsError, MountOptions,
};

#[derive(Debug, Default)]
pub struct DevFsDriver;

impl FileSystemDriver for DevFsDriver {
    fn mount(&self, _options: &MountOptions) -> Result<Arc<dyn FileSystem>, FsError> {
        Ok(Arc::new(DevFs))
    }
}

struct DevFs;

impl FileSystem for DevFs {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let device = match normalize(path).as_str() {
            "console" | "tty" | "tty0" => DeviceKind::Console,
            "screen" | "vga" => DeviceKind::Screen,
            "serial" | "ttyS0" => DeviceKind::Serial,
            "keyboard" | "kbd" => DeviceKind::Keyboard,
            "null" => DeviceKind::Null,
            "zero" => DeviceKind::Zero,
            _ => return Err(FsError::NotFound),
        };

        Ok(FileHandle::new(Box::new(DeviceFile { device })))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        if !normalize(path).is_empty() {
            return Err(FsError::NotFound);
        }

        Ok(vec![
            "console".to_string(),
            "tty".to_string(),
            "screen".to_string(),
            "serial".to_string(),
            "keyboard".to_string(),
            "null".to_string(),
            "zero".to_string(),
        ])
    }

    fn create(&self, _path: &str, _directory: bool) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn remove(&self, _path: &str) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        if normalize(path).is_empty() {
            return Ok(metadata(0o755, true));
        }

        self.open(path)?;
        Ok(metadata(0o666, false))
    }

    fn chmod(&self, _path: &str, _mode: u16) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn chown(&self, _path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }
}

#[derive(Clone, Copy)]
enum DeviceKind {
    Console,
    Screen,
    Serial,
    Keyboard,
    Null,
    Zero,
}

struct DeviceFile {
    device: DeviceKind,
}

impl FileOps for DeviceFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        match self.device {
            DeviceKind::Console => read_console(buf),
            DeviceKind::Serial => read_serial(buf),
            DeviceKind::Keyboard => read_keyboard(buf),
            DeviceKind::Null => Ok(0),
            DeviceKind::Zero => {
                buf.fill(0);
                Ok(buf.len())
            }
            DeviceKind::Screen => Err(FsError::Unsupported),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        match self.device {
            DeviceKind::Console => {
                write_screen(buf);
                write_serial(buf);
                Ok(buf.len())
            }
            DeviceKind::Screen => {
                write_screen(buf);
                Ok(buf.len())
            }
            DeviceKind::Serial => {
                write_serial(buf);
                Ok(buf.len())
            }
            DeviceKind::Null | DeviceKind::Zero => Ok(buf.len()),
            DeviceKind::Keyboard => Err(FsError::Unsupported),
        }
    }

    fn seek(&mut self, _pos: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn ioctl(&mut self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        match self.device {
            DeviceKind::Console | DeviceKind::Screen => screen_ioctl(request, arg, directory),
            _ => Err(FsError::Unsupported),
        }
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(metadata(0o666, false))
    }
}

fn normalize(path: &str) -> String {
    path.trim_matches('/').to_string()
}

fn metadata(mode: u16, is_dir: bool) -> FileMetadata {
    FileMetadata {
        uid: 0,
        gid: 0,
        mode,
        size: 0,
        is_dir,
    }
}

fn read_console(buf: &mut [u8]) -> Result<usize, FsError> {
    read_from(buf, || {
        KERNEL
            .keyboard_pop()
            .or_else(|| KERNEL.serial_read_byte())
            .map(normalize_input)
    })
}

fn read_serial(buf: &mut [u8]) -> Result<usize, FsError> {
    read_from(buf, || KERNEL.serial_read_byte().map(normalize_input))
}

fn read_keyboard(buf: &mut [u8]) -> Result<usize, FsError> {
    read_from(buf, || KERNEL.keyboard_pop().map(normalize_input))
}

fn read_from<F>(buf: &mut [u8], mut next_byte: F) -> Result<usize, FsError>
where
    F: FnMut() -> Option<u8>,
{
    let mut read = 0;

    while read < buf.len() {
        let Some(byte) = next_byte() else {
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

fn normalize_input(byte: u8) -> u8 {
    match byte {
        b'\n' => 13,
        0x7f => 0x08,
        byte => byte,
    }
}

fn write_screen(buf: &[u8]) {
    for byte in buf.iter().copied() {
        terminal_writechar(byte, 15);
    }
}

fn write_serial(buf: &[u8]) {
    for byte in buf.iter().copied() {
        KERNEL.serial_write_byte(byte);
    }
}

fn screen_ioctl(request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
    match request {
        TIOCGWINSZ => {
            if arg == 0 {
                return Err(FsError::InvalidArgument);
            }

            let Some((rows, cols)) =
                KERNEL.with_text(|text| text.map(|text| (text.rows() as u16, text.cols() as u16)))
            else {
                return Err(FsError::Unsupported);
            };

            let size = WinSize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };

            copy_string_to_task(
                directory,
                &size as *const WinSize as u32,
                arg,
                core::mem::size_of::<WinSize>() as u32,
            )
            .map_err(|_| FsError::IoError)?;

            Ok(0)
        }
        POLYOS_IOCTL_SCREEN_CLEAR => {
            clear_screen();
            Ok(0)
        }
        POLYOS_IOCTL_SCREEN_SET_COLOR => KERNEL.with_text(|text| {
            if let Some(text) = text {
                text.set_color(ColorCode::from(arg as u8));
                Ok(0)
            } else {
                Err(FsError::Unsupported)
            }
        }),
        POLYOS_IOCTL_SCREEN_DISABLE_CURSOR => {
            disable_cursor();
            Ok(0)
        }
        _ => Err(FsError::Unsupported),
    }
}
