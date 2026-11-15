//! Counts the bytes the editor allocates in a headless run, the bytes it
//! holds, and the processor time it takes. A scenario puts `%measure`
//! before some lines and `?memory` or `?time` after them, and
//! `sandbox` reads the counts at both.
//!
//! Time is the simple one: `editor(..)` adds the processor time its work
//! took. Allocations need the rest of this.
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
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};
use std::time::Duration;

use rustix::time::{ClockId, clock_gettime};

/// The sum of the sizes of all counted allocations so far. Freeing does
/// not reduce it: a copy that is made and dropped must still show.
static BYTES: AtomicU64 = AtomicU64::new(0);

/// The counted bytes taken and not yet given back.
///
/// A block is counted when it is taken and when it is given back by the
/// same rule, where the thread is at that moment. A block that the run
/// takes and the editor gives back, the events of a key say, takes this
/// down without having taken it up, so the number can be below zero and
/// only its change over some lines means anything. Such blocks are small
/// and the same for every size of input.
static HELD: AtomicI64 = AtomicI64::new(0);

/// The most `HELD` has been since `highest()` was last asked.
static HIGHEST: AtomicI64 = AtomicI64::new(0);

/// True once `Counting` has served an allocation, which means it is the
/// allocator of this binary. In a binary that uses another allocator
/// `BYTES` stays at zero, and without this flag every limit would pass.
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// The nanoseconds of processor time spent inside `editor(..)` so far.
static NANOS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// True while this thread's allocations are not counted: on the thread
    /// that reads the scenario, outside `editor(..)`. False on every other
    /// thread.
    static APART: Cell<bool> = const { Cell::new(false) };
}

/// The system's allocator, counting. Installed with `#[global_allocator]`.
pub struct Counting;

/// Whether this thread's allocations are counted now. `try_with` fails
/// while the thread's local storage is being destroyed, at the end of the
/// thread. An allocation made then is counted, as on any thread whose mark
/// is not set.
fn counted() -> bool {
    !APART.try_with(Cell::get).unwrap_or(false)
}

/// A block of `bytes` is taken from the system.
fn taken(bytes: usize) {
    // Written once and read after: a write on every allocation would have
    // every thread taking the flag's cache line from the others.
    if !INSTALLED.load(Relaxed) {
        INSTALLED.store(true, Relaxed);
    }
    if counted() {
        BYTES.fetch_add(bytes as u64, Relaxed);
        let held = HELD.fetch_add(bytes as i64, Relaxed) + bytes as i64;
        HIGHEST.fetch_max(held, Relaxed);
    }
}

/// A block of `bytes` is given back to the system.
fn given_back(bytes: usize) {
    if counted() {
        HELD.fetch_sub(bytes as i64, Relaxed);
    }
}

// SAFETY: every call is the system allocator's, with the arguments given.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        taken(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        taken(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        given_back(layout.size());
        unsafe { System.dealloc(pointer, layout) }
    }

    /// Counted as a new block of the whole new size taken and then the
    /// old one given back: the system may move the block, and then both
    /// are held while all of it is copied. A large buffer grown by a few
    /// bytes is a copy of the buffer, not a few bytes.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        taken(size);
        given_back(layout.size());
        unsafe { System.realloc(pointer, layout, size) }
    }
}

/// The bytes the editor has allocated so far. `None` when `Counting` is
/// not the allocator of this binary.
pub fn allocated() -> Option<u64> {
    INSTALLED.load(Relaxed).then(|| BYTES.load(Relaxed))
}

/// The bytes the editor holds now: taken and not given back. Only the
/// difference between two readings means something, see `HELD`.
pub fn held() -> i64 {
    HELD.load(Relaxed)
}

/// The most `held()` has been since this was last asked, and from now the
/// most is counted afresh. One mark for the whole process: whoever follows
/// several stretches at once asks here at every start and end of one, and
/// keeps the most of each stretch itself.
pub fn highest() -> i64 {
    let held = HELD.load(Relaxed);
    HIGHEST.swap(held, Relaxed).max(held)
}

/// Marks this thread as the one that reads the scenario: its allocations
/// are counted only inside `editor(..)`. The mark lasts until the returned
/// value is dropped.
pub fn harness() -> Restore {
    Restore {
        before: APART.replace(true),
    }
}

/// Runs `work` with this thread's allocations counted and the processor
/// time it took added to `spent()`. For the editor's work on the
/// scenario's thread: handling input, composing a frame.
pub fn editor<T>(work: impl FnOnce() -> T) -> T {
    let _restore = Restore {
        before: APART.replace(false),
    };
    let started = processor_time();
    let done = work();
    NANOS.fetch_add(processor_time().saturating_sub(started), Relaxed);
    done
}

/// The processor time spent inside `editor(..)` so far. The run's own work
/// between two lines is not in it.
pub fn spent() -> Duration {
    Duration::from_nanos(NANOS.load(Relaxed))
}

/// The nanoseconds of processor time this process has used, on all of its
/// threads. Processor time and not the time on the wall: a machine that
/// gives the core to another process for a while makes the wall's time
/// longer and this one no longer. All threads and not this one: a
/// background job of the editor works on its own thread while this one
/// waits for it in `wait`.
fn processor_time() -> u64 {
    let now = clock_gettime(ClockId::ProcessCPUTime);
    now.tv_sec as u64 * 1_000_000_000 + now.tv_nsec as u64
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
