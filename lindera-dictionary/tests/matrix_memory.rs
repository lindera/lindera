//! Checks how much memory `ConnectionCostMatrixBuilder::build` allocates for
//! a small `matrix.def` (#1366).
//!
//! The builder reads `matrix.def` in blocks of up to 64 MiB, and a block is no
//! larger than the file, so a small `matrix.def` must not cost a 64 MiB
//! buffer. A global allocator counts the bytes allocated at once. It counts
//! every thread of this test binary, which is why the binary holds this one
//! test only.

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write as _;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use lindera_dictionary::builder::connection_cost_matrix::ConnectionCostMatrixBuilderOptions;

/// Bytes allocated now.
static CURRENT: AtomicUsize = AtomicUsize::new(0);
/// The largest value `CURRENT` has had since the last reset.
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// The system allocator, counting the bytes it hands out.
struct Counting;

impl Counting {
    /// Adds `size` bytes to the current count and raises the peak.
    fn grow(size: usize) {
        let now = CURRENT.fetch_add(size, Ordering::Relaxed) + size;
        PEAK.fetch_max(now, Ordering::Relaxed);
    }

    /// Takes `size` bytes from the current count.
    fn shrink(size: usize) {
        CURRENT.fetch_sub(size, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            Self::grow(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            Self::grow(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        Self::shrink(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size >= layout.size() {
                Self::grow(new_size - layout.size());
            } else {
                Self::shrink(layout.size() - new_size);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The most bytes the build of the 100 x 100 matrix below may allocate at
/// once: far above the text (about 110 KB), the matrix (20 KB) and the piece
/// of `matrix.mtx` written at a time (128 KiB), far below a 64 MiB block.
const LIMIT: usize = 4 << 20; // 4 MiB

#[test]
fn test_small_matrix_def_is_not_read_into_a_whole_block() {
    let (forward, backward) = (100u32, 100u32);
    let mut matrix = format!("{forward} {backward}\n");
    for b in 0..backward {
        for f in 0..forward {
            let cost = ((f * 7 + b * 13) % 2000) as i32 - 1000;
            writeln!(matrix, "{f} {b} {cost}").unwrap();
        }
    }
    let input = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::write(input.path().join("matrix.def"), &matrix).unwrap();

    let before = CURRENT.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    ConnectionCostMatrixBuilderOptions::default()
        .builder()
        .build(input.path(), output.path())
        .unwrap();
    let allocated = PEAK.load(Ordering::Relaxed) - before;

    assert!(
        allocated < LIMIT,
        "building a {}-byte matrix.def allocated {allocated} bytes at once, \
         not less than {LIMIT}",
        matrix.len()
    );
    // The header of three values, then every cell, two bytes each.
    let mtx = fs::read(output.path().join("matrix.mtx")).unwrap();
    assert_eq!(mtx.len(), (3 + (forward * backward) as usize) * 2);
}
