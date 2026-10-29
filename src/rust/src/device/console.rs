use alloc::boxed::Box;

use crate::{
    device::{
        control::{
            POLYOS_IOCTL_SCREEN_CLEAR, POLYOS_IOCTL_SCREEN_DISABLE_CURSOR,
            POLYOS_IOCTL_SCREEN_SET_COLOR,
        },
        screen::{ColorCode, SCREEN_DRIVER},
    },
    fs::{FileHandle, FileMetadata, FileOps, FsError},
    memory::PageDirectory,
    print::{clear_screen, disable_cursor},
};

struct ConsoleFile;

impl FileOps for ConsoleFile {
    fn read_nonblocking(&mut self, out: &mut [u8]) -> Result<usize, FsError> {
        super::terminal::read_nonblocking(out)
    }
    fn poll(&self, events: u16) -> u16 {
        super::terminal::poll(events)
    }
    fn read(&mut self, out: &mut [u8]) -> Result<usize, FsError> {
        super::terminal::read(out)
    }
    fn write(&mut self, data: &[u8]) -> Result<usize, FsError> {
        super::terminal::write(data)
    }
    fn seek(&mut self, _pos: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }

    fn ioctl(&mut self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        if let Some(result) = super::terminal::ioctl(request, arg, directory)? {
            return Ok(result);
        }
        match request {
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

pub fn open_console() -> FileHandle {
    FileHandle::new(Box::new(ConsoleFile))
}

crate::register_device_node!(
    CONSOLE_DEVICE_NODE_REG,
    ["console", "tty", "tty0"],
    open_console
);
