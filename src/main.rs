mod document;
mod key;
mod layout;
mod movement;
mod render;
mod selection;
mod terminal;
mod text;
mod view;

use std::io;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::document::Document;
use crate::key::{Key, KeyCode, Mods};
use crate::movement::{Direction, Motion};
use crate::selection::{Selection, SelectionSet};
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
        render::draw(&mut out, document, &view, &status(term, document, &view))?;

        match event::read()? {
            Event::Key(event) => match Key::from_event(&event) {
                Some(key) if is_quit(key) => return Ok(()),
                Some(key) => handle_key(key, &mut view, document),
                None => {}
            },
            Event::Mouse(mouse) => handle_mouse(mouse, &mut view, document),
            _ => {}
        }
    }
}

fn is_quit(key: Key) -> bool {
    key.code == KeyCode::Char('q') && key.mods == Mods::CTRL
}

/// Temporary bindings: these become commands once the keymap exists.
fn handle_key(key: Key, view: &mut View, document: &Document) {
    use Direction::{Backward, Forward};

    let text = &document.text;
    let extend = key.mods.contains(Mods::SHIFT);
    let ctrl = key.mods.contains(Mods::CTRL);
    let page = view.height.saturating_sub(1);

    match key.code {
        KeyCode::Left if ctrl => view.move_selections(text, Motion::Word(Backward), extend),
        KeyCode::Right if ctrl => view.move_selections(text, Motion::Word(Forward), extend),
        KeyCode::Left => view.move_selections(text, Motion::Grapheme(Backward), extend),
        KeyCode::Right => view.move_selections(text, Motion::Grapheme(Forward), extend),
        KeyCode::Up => view.move_selections(text, Motion::Row(Backward), extend),
        KeyCode::Down => view.move_selections(text, Motion::Row(Forward), extend),
        KeyCode::Home if ctrl => view.move_selections(text, Motion::Document(Backward), extend),
        KeyCode::End if ctrl => view.move_selections(text, Motion::Document(Forward), extend),
        KeyCode::Home => view.move_selections(text, Motion::LineEdge(Backward), extend),
        KeyCode::End => view.move_selections(text, Motion::LineEdge(Forward), extend),
        KeyCode::PageUp => {
            for _ in 0..page {
                view.move_selections(text, Motion::Row(Backward), extend);
            }
        }
        KeyCode::PageDown => {
            for _ in 0..page {
                view.move_selections(text, Motion::Row(Forward), extend);
            }
        }
        KeyCode::Esc => view.selections.collapse_to_primary(),
        _ => {}
    }
}

fn handle_mouse(mouse: MouseEvent, view: &mut View, document: &Document) {
    let text = &document.text;
    let column = (mouse.column as usize).saturating_sub(view.gutter);
    let row = mouse.row as usize;

    match mouse.kind {
        MouseEventKind::ScrollUp => view.scroll_lines(-MOUSE_SCROLL_LINES, text),
        MouseEventKind::ScrollDown => view.scroll_lines(MOUSE_SCROLL_LINES, text),
        MouseEventKind::Down(MouseButton::Left) if row < view.height => {
            let offset = view.offset_at_screen(text, row, column);
            let cursor = Selection::cursor(offset);
            match mouse.modifiers.contains(KeyModifiers::ALT) {
                true => view.selections.push(cursor),
                false => view.selections = SelectionSet::single(offset),
            }
        }
        MouseEventKind::Drag(MouseButton::Left) if row < view.height => {
            view.selections = Selection {
                anchor: view.selections.primary().anchor,
                head: view.offset_at_screen(text, row, column),
                goal_column: None,
            }
            .into();
            view.scroll_to_cursor(text);
        }
        _ => {}
    }
}

fn status(term: &Terminal, document: &Document, view: &View) -> String {
    let path = document
        .path
        .as_ref()
        .map_or("[scratch]".to_owned(), |path| path.display().to_string());
    let keyboard = match term.keyboard_enhanced {
        true => "enhanced",
        false => "legacy (some keys unavailable)",
    };

    let head = view.selections.primary().head;
    let line = document.text.line_of(head);
    let column = head - document.text.line_start(line);
    let cursors = match view.selections.len() {
        1 => String::new(),
        count => format!(" · {count} cursors"),
    };

    format!(
        " {path} · {}:{}{cursors} · {} lines · {:?} · keyboard: {keyboard}",
        line + 1,
        column + 1,
        document.text.line_count(),
        document.line_ending,
    )
}
