#![allow(clippy::missing_safety_doc)]

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let dst_slice = unsafe { core::slice::from_raw_parts_mut(dst, n) };
    let src_slice = unsafe { core::slice::from_raw_parts(src, n) };
    if dst_slice.as_ptr() < src_slice.as_ptr() {
        dst_slice[..n].copy_from_slice(&src_slice[..n]);
    } else {
        for i in (0..n).rev() {
            dst_slice[i] = src_slice[i];
        }
    }
    dst_slice.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let dst = unsafe { core::slice::from_raw_parts_mut(dst, n) };
    let src = unsafe { core::slice::from_raw_parts(src, n) };
    dst[..n].copy_from_slice(&src[..n]);
    dst.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let s = unsafe { core::slice::from_raw_parts_mut(s, n) };

    for byte in s.iter_mut() {
        *byte = c as u8;
    }

    s.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let s1 = unsafe { core::slice::from_raw_parts(a, n) };
    let s2 = unsafe { core::slice::from_raw_parts(b, n) };

    for i in 0..n {
        if s1[i] != s2[i] {
            return (s1[i] as i32) - (s2[i] as i32);
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(a: *const u8) -> i32 {
    let mut len = 0;
    unsafe {
        while *a.add(len) != 0 {
            len += 1;
        }
    }
    len as i32
}
