use alloc::boxed::Box;

use crate::{
    device::{
        control::{
            POLYOS_IOCTL_SCREEN_CLEAR, POLYOS_IOCTL_SCREEN_DISABLE_CURSOR,
            POLYOS_IOCTL_SCREEN_SET_COLOR, TIOCGWINSZ, WinSize,
        },
        keyboard::KEYBOARD_DRIVER,
        screen::{ColorCode, SCREEN_DRIVER},
        serial::SERIAL_DRIVER,
    },
    fs::{FileHandle, FileMetadata, FileOps, FsError},
    memory::PageDirectory,
    print::{clear_screen, disable_cursor, terminal_writechar},
    schedule::task::copy_string_to_task,
};

struct ConsoleFile;

impl FileOps for ConsoleFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        read_from(buf, || {
            KEYBOARD_DRIVER
                .read_byte()
                .or_else(|| SERIAL_DRIVER.read_byte())
                .map(normalize_input)
        })
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        for byte in buf.iter().copied() {
            terminal_writechar(byte, 15);
            SERIAL_DRIVER.write_byte(byte);
        }

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
            POLYOS_IOCTL_SCREEN_SET_COLOR => {
                SCREEN_DRIVER.set_color(ColorCode::from(arg as u8));
                Ok(0)
            }
            POLYOS_IOCTL_SCREEN_DISABLE_CURSOR => {
                disable_cursor();
                Ok(0)
            }
            _ => Err(FsError::Unsupported),
        }
    }

    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(metadata(0o666, false))
    }
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

pub fn open_console() -> FileHandle {
    FileHandle::new(Box::new(ConsoleFile))
}

crate::register_device_node!(CONSOLE_DEVICE_NODE_REG, ["console", "tty", "tty0"], open_console);
