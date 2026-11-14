//! The editor as a library. `main.rs` is only the process around it: without
//! this split the module tree would belong to the binary, and an integration
//! test could not reach any of it.
//!
//! Four layers, each using only its own and those below:
//! `text` — a buffer's contents and positions in it, nothing about files or
//! screens; `buffer` — an open file with its history; `editor` — commands,
//! views, panels, the one boundary; `term` — the terminal and the process
//! around the editor.

// A test comparing hits to `[0..1]` means exactly that array.
#![cfg_attr(test, allow(clippy::single_range_in_vec_init))]

/// The unit tests count allocations as the scenario tests do, so that the
/// sandbox's own tests can state a limit.
#[cfg(all(test, debug_assertions))]
#[global_allocator]
static ALLOCATOR: term::meter::Counting = term::meter::Counting;

pub mod buffer;
pub mod editor;
pub mod term;
pub mod text;
