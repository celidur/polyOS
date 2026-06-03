use core::ffi::c_void;

pub fn pipe() -> Result<(i32, i32), i32> {
    let mut fds = [0_i32; 2];
    let result = unsafe { crate::bindings::pipe(fds.as_mut_ptr()) };

    if result == 0 {
        Ok((fds[0], fds[1]))
    } else {
        Err(result)
    }
}

pub fn read(fd: i32, buf: &mut [u8]) -> Result<usize, isize> {
    let result =
        unsafe { crate::bindings::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };

    if result >= 0 {
        Ok(result as usize)
    } else {
        Err(result)
    }
}

pub fn write(fd: i32, buf: &[u8]) -> Result<usize, isize> {
    let result =
        unsafe { crate::bindings::write(fd, buf.as_ptr() as *const c_void, buf.len()) };

    if result >= 0 {
        Ok(result as usize)
    } else {
        Err(result)
    }
}

pub fn close(fd: i32) -> Result<(), i32> {
    let result = unsafe { crate::bindings::close(fd) };

    if result == 0 {
        Ok(())
    } else {
        Err(result)
    }
}
