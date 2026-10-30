//! Shared console line discipline and terminal session policy.
use super::{control::WinSize, keyboard::KEYBOARD_DRIVER, serial::SERIAL_DRIVER};
use crate::{
    fs::FsError,
    kernel::KERNEL,
    memory::{PageDirectory, user},
    schedule::process::SIG_IGN,
};
use alloc::{collections::VecDeque, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub input: u32,
    pub output: u32,
    pub control: u32,
    pub local: u32,
    pub line: u8,
    pub chars: [u8; 19],
}
impl Default for Termios {
    fn default() -> Self {
        let mut chars = [0; 19];
        chars[0] = 3;
        chars[1] = 28;
        chars[2] = 127;
        chars[3] = 21;
        chars[4] = 4;
        chars[6] = 1;
        chars[10] = 26;
        Self {
            input: 256,
            output: 5,
            control: 0x8b0,
            local: 1 | 2 | 8 | 16 | 32,
            line: 0,
            chars,
        }
    }
}
struct Terminal {
    settings: Termios,
    edit: Vec<u8>,
    ready: VecDeque<Option<u8>>,
    session: Option<u32>,
    foreground: u32,
    size: WinSize,
}
impl Default for Terminal {
    fn default() -> Self {
        Self {
            settings: Default::default(),
            edit: Vec::new(),
            ready: VecDeque::new(),
            session: None,
            foreground: 0,
            size: WinSize {
                ws_row: 25,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        }
    }
}
lazy_static! {
    static ref TERMINAL: Mutex<Terminal> = Mutex::new(Terminal::default());
}
fn attach() {
    let process = KERNEL.with_task_manager(|tm| tm.get_current().map(|t| t.read().process.clone()));
    if let Some(process) = process {
        let c = process.control.lock();
        let mut tty = TERMINAL.lock();
        if tty.session.is_none() {
            tty.session = Some(c.sid);
            tty.foreground = c.pgid;
        }
    }
}
fn emit(bytes: &[u8], post: bool) {
    for &b in bytes {
        if post && b == b'\n' {
            SERIAL_DRIVER.write(b"\r");
        }
        SERIAL_DRIVER.write(&[b]);
        crate::print::terminal_writechar(b, 15);
    }
}
pub fn write(bytes: &[u8]) -> Result<usize, FsError> {
    attach();
    if TERMINAL.lock().settings.local & 256 != 0 {
        job_check(22)?;
    }
    let post = TERMINAL.lock().settings.output & 5 == 5;
    emit(bytes, post);
    Ok(bytes.len())
}
impl Terminal {
    fn finish(&mut self) {
        for b in self.edit.drain(..) {
            self.ready.push_back(Some(b));
        }
        self.ready.push_back(None);
    }
    fn input(&mut self, mut b: u8, echo: &mut Vec<u8>, signals: &mut Vec<u32>) {
        let s = self.settings;
        if b == b'\r' {
            if s.input & 128 != 0 {
                return;
            }
            if s.input & 256 != 0 {
                b = b'\n';
            }
        } else if b == b'\n' && s.input & 64 != 0 {
            b = b'\r';
        }
        if s.local & 1 != 0 {
            for (index, signal) in [(0, 2), (1, 3), (10, 20)] {
                if s.chars[index] != 0 && b == s.chars[index] {
                    self.ready.clear();
                    self.edit.clear();
                    if s.local & 8 != 0 {
                        echo.extend_from_slice(&[b'^', if b < 32 { b + 64 } else { b'?' }, b'\n']);
                    }
                    signals.push(signal);
                    return;
                }
            }
        }
        if s.local & 2 != 0 {
            if s.chars[2] != 0 && b == s.chars[2] {
                if self.edit.pop().is_some() && s.local & 8 != 0 {
                    if s.local & 16 != 0 {
                        echo.extend_from_slice(b"\x08 \x08");
                    } else {
                        echo.push(b);
                    }
                }
                return;
            }
            if s.chars[3] != 0 && b == s.chars[3] {
                self.edit.clear();
                if s.local & 8 != 0 && s.local & 32 != 0 {
                    echo.push(b'\n');
                }
                return;
            }
            if s.chars[4] != 0 && b == s.chars[4] {
                self.finish();
                return;
            }
            if self.edit.len() < 4095 {
                self.edit.push(b);
                if s.local & 8 != 0 {
                    echo.push(b);
                }
            }
            if b == b'\n' || (s.chars[11] != 0 && b == s.chars[11]) {
                self.finish();
            }
        } else if self.ready.len() < 8192 {
            self.ready.push_back(Some(b));
            if s.local & 8 != 0 {
                echo.push(b);
            }
        }
    }
}
fn discard_input() {
    for _ in 0..8192 {
        if KEYBOARD_DRIVER
            .read_byte()
            .or_else(|| SERIAL_DRIVER.read())
            .is_none()
        {
            break;
        }
    }
}
pub fn pump() {
    let mut echo = Vec::new();
    let mut signals = Vec::new();
    let (group, post) = {
        let mut tty = TERMINAL.lock();
        for _ in 0..512 {
            let Some(b) = KEYBOARD_DRIVER
                .read_byte()
                .map(|b| if b == 8 { 127 } else { b })
                .or_else(|| SERIAL_DRIVER.read())
            else {
                break;
            };
            if tty.ready.len() < 8192 {
                tty.input(b, &mut echo, &mut signals);
            }
        }
        (tty.foreground, tty.settings.output & 5 == 5)
    };
    emit(&echo, post);
    for signal in signals {
        KERNEL.with_process_manager(|pm| pm.signal_group(group, signal));
    }
}
fn job_check(signal: u32) -> Result<(), FsError> {
    let context = KERNEL.with_task_manager(|tm| {
        tm.get_current().map(|t| {
            let t = t.read();
            (t.process.clone(), t.signals.blocked)
        })
    });
    let Some((process, mask)) = context else {
        return Err(FsError::IoError);
    };
    let c = process.control.lock();
    let (session, foreground) = {
        let tty = TERMINAL.lock();
        (tty.session, tty.foreground)
    };
    if session != Some(c.sid) {
        return Err(FsError::IoError);
    }
    if foreground == c.pgid {
        return Ok(());
    }
    let ignored = process
        .get_signal_action(signal)
        .is_some_and(|a| a.handler == SIG_IGN);
    let group = c.pgid;
    drop(c);
    if ignored || mask & (1 << (signal - 1)) != 0 {
        if signal == 22 {
            return Ok(());
        }
        return Err(FsError::IoError);
    }
    KERNEL.with_process_manager(|pm| pm.signal_group(group, signal));
    Err(FsError::Interrupted)
}
pub fn read(out: &mut [u8]) -> Result<usize, FsError> {
    read_mode(out, false)
}
pub fn read_nonblocking(out: &mut [u8]) -> Result<usize, FsError> {
    read_mode(out, true)
}
fn read_mode(out: &mut [u8], nonblocking: bool) -> Result<usize, FsError> {
    if out.is_empty() {
        return Ok(0);
    }
    attach();
    job_check(21)?;
    pump();
    let mut tty = TERMINAL.lock();
    let canonical = tty.settings.local & 2 != 0;
    if tty.ready.is_empty() {
        return if !canonical && tty.settings.chars[6] == 0 {
            Ok(0)
        } else {
            Err(FsError::WouldBlock)
        };
    }
    if !canonical && !nonblocking {
        let available = tty.ready.iter().filter(|b| b.is_some()).count();
        if available < out.len().min(tty.settings.chars[6] as usize) {
            return Err(FsError::WouldBlock);
        }
    }
    let mut count = 0;
    while count < out.len() {
        match tty.ready.pop_front() {
            Some(Some(b)) => {
                out[count] = b;
                count += 1;
            }
            Some(None) if canonical => break,
            Some(None) => {}
            None => break,
        }
    }
    // A full buffer can end immediately before its record delimiter. If the
    // loop already consumed a delimiter, the next one belongs to another EOF.
    if canonical && count == out.len() && tty.ready.front() == Some(&None) {
        tty.ready.pop_front();
    }
    Ok(count)
}
pub fn poll(events: u16) -> u16 {
    attach();
    pump();
    let tty = TERMINAL.lock();
    let readable = if tty.settings.local & 2 != 0 {
        !tty.ready.is_empty()
    } else {
        let minimum = (tty.settings.chars[6] as usize).max(1);
        tty.ready.iter().filter(|b| b.is_some()).count() >= minimum
    };
    (events & 4) | if readable { events & 1 } else { 0 }
}
fn output<T>(directory: &PageDirectory, ptr: u32, value: &T) -> Result<u32, FsError> {
    user::write_value(directory, ptr, value).map_err(|_| FsError::BadAddress)?;
    Ok(0)
}
pub fn ioctl(request: u32, arg: u32, directory: &PageDirectory) -> Result<Option<u32>, FsError> {
    attach();
    match request {
        0x5401 => {
            let settings = TERMINAL.lock().settings;
            output(directory, arg, &settings).map(Some)
        }
        0x5402 | 0x5403 | 0x5404 => {
            job_check(22)?;
            let mut settings = Termios::default();
            user::copy_from_user(directory, arg, &mut settings as *mut _ as *mut u8, 36)
                .map_err(|_| FsError::BadAddress)?;
            if settings.input & !(64 | 128 | 256) != 0
                || settings.output & !5 != 0
                || settings.local & !(1 | 2 | 8 | 16 | 32 | 256) != 0
                || settings.control != 0x8b0
                || settings.line != 0
                || settings.chars[5] != 0
            {
                return Err(FsError::NotSupported);
            }
            if request == 0x5404 {
                discard_input();
            }
            let mut tty = TERMINAL.lock();
            if request == 0x5404 {
                tty.ready.clear();
                tty.edit.clear();
            } else if tty.settings.local & 2 != 0 && settings.local & 2 == 0 {
                // Canonical record delimiters are bookkeeping, not input bytes.
                // Keeping them in raw mode can surface a stale EOF when a
                // readline user restores canonical mode for its child.
                tty.ready.retain(|byte| byte.is_some());
                let edit = core::mem::take(&mut tty.edit);
                for b in edit {
                    tty.ready.push_back(Some(b));
                }
            }
            tty.settings = settings;
            Ok(Some(0))
        }
        0x540b => {
            if arg > 2 {
                return Err(FsError::InvalidArgument);
            }
            if arg != 1 {
                discard_input();
                let mut tty = TERMINAL.lock();
                tty.ready.clear();
                tty.edit.clear();
            }
            Ok(Some(0))
        }
        0x540f => {
            let sid = KERNEL.with_task_manager(|tm| {
                tm.get_current()
                    .map(|t| t.read().process.control.lock().sid)
            });
            if TERMINAL.lock().session != sid {
                return Err(FsError::Unsupported);
            }
            let tty = TERMINAL.lock();
            let group = tty.foreground;
            drop(tty);
            output(directory, arg, &group).map(Some)
        }
        0x5410 => {
            job_check(22)?;
            let mut group = 0u32;
            user::copy_from_user(directory, arg, &mut group as *mut u32 as *mut u8, 4)
                .map_err(|_| FsError::BadAddress)?;
            if group > i32::MAX as u32 {
                return Err(FsError::InvalidArgument);
            }
            let caller = KERNEL
                .with_task_manager(|tm| tm.get_current().map(|t| t.read().process.clone()))
                .ok_or(FsError::IoError)?;
            let sid = caller.control.lock().sid;
            if KERNEL.with_process_manager(|pm| pm.group_session(group)) != Some(sid) {
                return Err(FsError::PermissionDenied);
            }
            let mut tty = TERMINAL.lock();
            if tty.session != Some(sid) {
                return Err(FsError::Unsupported);
            }
            tty.foreground = group;
            Ok(Some(0))
        }
        0x5413 => {
            let size = TERMINAL.lock().size;
            output(directory, arg, &size).map(Some)
        }
        0x5414 => {
            let mut size = WinSize::default();
            user::copy_from_user(directory, arg, &mut size as *mut _ as *mut u8, 8)
                .map_err(|_| FsError::BadAddress)?;
            if size.ws_row == 0 || size.ws_col == 0 {
                return Err(FsError::InvalidArgument);
            }
            let group = {
                let mut tty = TERMINAL.lock();
                tty.size = size;
                tty.foreground
            };
            KERNEL.with_process_manager(|pm| pm.signal_group(group, 28));
            Ok(Some(0))
        }
        _ => Ok(None),
    }
}
