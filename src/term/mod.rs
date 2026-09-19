//! The terminal and the process around the editor: reading events, drawing
//! frames, owning raw mode. The only layer that talks to crossterm for
//! anything but a key's name.

pub mod app;
pub mod grid;
pub mod render;
pub mod system_clipboard;
pub mod terminal;
