//! The editor as a library. `main.rs` is only the process around it: without
//! this split the module tree would belong to the binary, and an integration
//! test could not reach any of it.

pub mod anchor;
pub mod app;
pub mod buffer;
pub mod clipboard;
pub mod command;
pub mod editor;
pub mod fuzzy;
pub mod grid;
pub mod indent;
pub mod job;
pub mod key;
pub mod keymap;
pub mod language;
pub mod layout;
pub mod movement;
pub mod panel;
pub mod picker;
pub mod prompt;
pub mod render;
pub mod screen;
pub mod search;
pub mod selection;
pub mod terminal;
pub mod text;
pub mod transaction;
pub mod view;
pub mod workspace;
