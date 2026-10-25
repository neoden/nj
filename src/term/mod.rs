//! The terminal and the process around the editor: reading events, drawing
//! frames, owning raw mode. The only layer that talks to crossterm.

pub mod app;
pub mod dump;
pub mod grid;
pub mod input;
pub mod render;
pub mod scenario;
pub mod system_clipboard;
pub mod terminal;
