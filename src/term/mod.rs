//! The terminal and the process around the editor: reading events, drawing
//! frames, owning raw mode. The only layer that talks to crossterm.

pub mod app;
pub mod grid;
pub mod input;
pub mod launch;
#[cfg(debug_assertions)]
pub mod meter;
pub mod render;
#[cfg(debug_assertions)]
pub mod sandbox;
pub mod scenario;
pub mod screen;
pub mod socket;
pub mod system_clipboard;
pub mod terminal;
pub mod theme;
