//! The editor as a library. `main.rs` is only the process around it: without
//! this split the module tree would belong to the binary, and an integration
//! test could not reach any of it.

pub mod app;
pub mod clipboard;
pub mod document;
pub mod editor;
pub mod fuzzy;
pub mod indent;
pub mod key;
pub mod language;
pub mod layout;
pub mod movement;
pub mod panel;
pub mod picker;
pub mod prompt;
pub mod render;
pub mod selection;
pub mod terminal;
pub mod text;
pub mod transaction;
pub mod view;
