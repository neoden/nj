//! Counts the bytes the editor allocates in a headless run. A scenario
//! puts `%allocated` and `?allocated` around some lines, and `sandbox`
//! reads the count at both.
//!
//! A Rust process has one allocator, so the editor cannot be given a
//! counting one while the test keeps the ordinary one. `Counting` is the
//! allocator of the whole test binary and decides per thread what to count:
//!
//! - The thread that reads the scenario calls `harness()` once. After that
//!   its allocations are counted only inside `editor(..)`.
//! - Every other thread is counted always. The editor starts those threads
//!   for its background jobs.
//!
//! Test machinery, so debug builds only.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

/// The sum of the sizes of all counted allocations so far. Freeing does
/// not reduce it: a copy that is made and dropped must still show.
static BYTES: AtomicU64 = AtomicU64::new(0);

/// True once `Counting` has served an allocation, which means it is the
/// allocator of this binary. In a binary that uses another allocator
/// `BYTES` stays at zero, and without this flag every limit would pass.
static INSTALLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// True while this thread's allocations are not counted: on the thread
    /// that reads the scenario, outside `editor(..)`. False on every other
    /// thread.
    static APART: Cell<bool> = const { Cell::new(false) };
}

/// The system's allocator, counting. Installed with `#[global_allocator]`.
pub struct Counting;

fn count(bytes: usize) {
    // Written once and read after: a write on every allocation would have
    // every thread taking the flag's cache line from the others.
    if !INSTALLED.load(Relaxed) {
        INSTALLED.store(true, Relaxed);
    }
    // `try_with` fails while the thread's local storage is being destroyed,
    // at the end of the thread. An allocation made then is counted, as on
    // any thread whose mark is not set.
    if !APART.try_with(Cell::get).unwrap_or(false) {
        BYTES.fetch_add(bytes as u64, Relaxed);
    }
}

// SAFETY: every call is the system allocator's, with the arguments given.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    /// Counted as a new block of the whole new size: the system may move
    /// the block, and then all of it is copied. A large buffer grown by a
    /// few bytes is a copy of the buffer, not a few bytes.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

/// The bytes the editor has allocated so far. `None` when `Counting` is
/// not the allocator of this binary.
pub fn allocated() -> Option<u64> {
    INSTALLED.load(Relaxed).then(|| BYTES.load(Relaxed))
}

/// Marks this thread as the one that reads the scenario: its allocations
/// are counted only inside `editor(..)`. The mark lasts until the returned
/// value is dropped.
pub fn harness() -> Restore {
    Restore {
        before: APART.replace(true),
    }
}

/// Runs `work` with this thread's allocations counted. For the editor's
/// work on the scenario's thread: handling input, composing a frame.
pub fn editor<T>(work: impl FnOnce() -> T) -> T {
    let _restore = Restore {
        before: APART.replace(false),
    };
    work()
}

/// Sets this thread's mark back to what it was, when dropped. Done in
/// `drop` so that it also happens when the work panics: a test that
/// catches the panic continues with the mark it had before.
pub struct Restore {
    before: bool,
}

impl Drop for Restore {
    fn drop(&mut self) {
        APART.set(self.before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_put_back_when_the_editors_work_panics() {
        let _harness = harness();
        let caught = std::panic::catch_unwind(|| editor(|| panic!("in the editor")));
        assert!(caught.is_err());
        assert!(APART.get());
    }
}
