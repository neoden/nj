//! The editor as a library. `main.rs` is only the process around it: without
//! this split the module tree would belong to the binary, and an integration
//! test could not reach any of it.
//!
//! Four layers, each using only its own and those below:
//! `text` — a buffer's contents and positions in it, nothing about files or
//! screens; `buffer` — an open file with its history; `editor` — commands,
//! views, panels, the one boundary; `term` — the terminal and the process
//! around the editor.

pub mod buffer;
pub mod editor;
pub mod term;
pub mod text;
