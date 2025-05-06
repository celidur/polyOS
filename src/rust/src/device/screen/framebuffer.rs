#![allow(dead_code)]
/// The framebuffer.
///
/// It's a special memory buffer that mapped from the device memory.
pub struct FrameBuffer<'a, T = u8> {
    _raw: &'a mut [T],
}

impl<'a, T> FrameBuffer<'a, T> {
    /// Use the given raw pointer and size as the framebuffer.
    ///
    /// # Safety
    ///
    /// Caller must insure that the given memory region is valid and accessible.
    pub unsafe fn from_raw_parts_mut(ptr: *mut T, len: usize) -> Self {
        Self {
            _raw: unsafe { core::slice::from_raw_parts_mut(ptr, len) },
        }
    }

    /// Use the given slice as the framebuffer.
    pub fn from_slice(slice: &'a mut [T]) -> Self {
        Self { _raw: slice }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self._raw
    }

    pub fn as_slice(&self) -> &[T] {
        self._raw
    }

    pub fn len(&self) -> usize {
        self._raw.len()
    }
}

impl<'a, T: Copy> FrameBuffer<'a, T> {
    /// Safely copies a region of the framebuffer from `src` to `dst` with `size` elements.
    ///
    /// # Panics
    ///
    /// Panics if the source or destination ranges are out of bounds,
    /// or if the ranges overlap in a way not allowed by `copy_from_slice`.
    pub fn copy_region(&mut self, src: usize, dst: usize, size: usize) {
        let len = self.len();
        assert!(src + size <= len, "Source range out of bounds");
        assert!(dst + size <= len, "Destination range out of bounds");

        // If the regions overlap in a problematic way, panic
        if src != dst {
            let src_range = src..src + size;
            let dst_range = dst..dst + size;

            let overlaps = src_range.start < dst_range.end && dst_range.start < src_range.end;
            assert!(!overlaps, "Source and destination ranges overlap");
        }

        let raw = self.as_mut_slice();

        let (first, second) = if src < dst {
            raw.split_at_mut(dst)
        } else {
            raw.split_at_mut(src)
        };

        let (src_slice, dst_slice) = if src < dst {
            (&first[src..src + size], &mut second[0..size])
        } else {
            (&second[0..size], &mut first[dst..dst + size])
        };

        dst_slice.copy_from_slice(src_slice);
    }
}
