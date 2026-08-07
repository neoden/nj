mod document;
mod key;
mod layout;
mod render;
mod terminal;
mod text;
mod view;

use std::io;

use anyhow::Result;
use crossterm::event::{self, Event, MouseEventKind};

use crate::document::Document;
use crate::key::{Key, KeyCode, Mods};
use crate::terminal::Terminal;
use crate::view::View;

const MOUSE_SCROLL_LINES: isize = 3;

fn main() -> Result<()> {
    terminal::install_panic_hook();

    let document = match std::env::args().nth(1) {
        Some(path) => Document::open(path)?,
        None => Document::default(),
    };

    let term = Terminal::enter()?;
    let result = run(&term, &document);
    drop(term);
    result
}

fn run(term: &Terminal, document: &Document) -> Result<()> {
    let mut out = io::stdout();
    let mut view = View::default();

    loop {
        let (width, height) = crossterm::terminal::size()?;
        view.fit(
            width as usize,
            height as usize,
            document.text.line_count(),
        );
        render::draw(&mut out, document, &view, &status(term, document))?;

        match event::read()? {
            Event::Key(event) => match Key::from_event(&event) {
                Some(key) if is_quit(key) => return Ok(()),
                Some(key) => scroll(key, &mut view, document),
                None => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    view.scroll_lines(-MOUSE_SCROLL_LINES, &document.text)
                }
                MouseEventKind::ScrollDown => {
                    view.scroll_lines(MOUSE_SCROLL_LINES, &document.text)
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn is_quit(key: Key) -> bool {
    key.code == KeyCode::Char('q') && key.mods == Mods::CTRL
}

/// Temporary bindings: these become commands once the keymap exists.
fn scroll(key: Key, view: &mut View, document: &Document) {
    let text = &document.text;
    let page = view.height.saturating_sub(1) as isize;
    let ctrl = key.mods == Mods::CTRL;

    match key.code {
        KeyCode::Up => view.scroll_lines(-1, text),
        KeyCode::Down => view.scroll_lines(1, text),
        KeyCode::PageUp => view.scroll_lines(-page, text),
        KeyCode::PageDown => view.scroll_lines(page, text),
        KeyCode::Home if ctrl => view.scroll_to_top(),
        KeyCode::End if ctrl => view.scroll_to_bottom(text),
        KeyCode::Home => view.left_column = 0,
        KeyCode::Left => view.scroll_columns(-1, text),
        KeyCode::Right => view.scroll_columns(1, text),
        _ => {}
    }
}

fn status(term: &Terminal, document: &Document) -> String {
    let path = document
        .path
        .as_ref()
        .map_or("[scratch]".to_owned(), |path| path.display().to_string());
    let keyboard = match term.keyboard_enhanced {
        true => "enhanced",
        false => "legacy (some keys unavailable)",
    };

    format!(
        " {path} · {} lines · {:?} · keyboard: {keyboard} · ctrl+q to quit",
        document.text.line_count(),
        document.line_ending,
    )
}
