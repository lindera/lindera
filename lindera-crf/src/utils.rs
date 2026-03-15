pub trait FromU32 {
    fn from_u32(src: u32) -> Self;
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl FromU32 for usize {
    #[inline(always)]
    fn from_u32(src: u32) -> Self {
        // Safety: Since the pointer width is guaranteed to be 32 or 64,
        // the following process always succeeds.
        unsafe { Self::try_from(src).unwrap_unchecked() }
    }
}
