mod document;
mod editor;
mod key;
mod layout;
mod movement;
mod render;
mod selection;
mod terminal;
mod text;
mod transaction;
mod view;

use std::io;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::document::Document;
use crate::editor::Editor;
use crate::key::{Key, KeyCode, Mods};
use crate::movement::{Direction, Motion};
use crate::selection::{Selection, SelectionSet};
use crate::terminal::Terminal;

const MOUSE_SCROLL_LINES: isize = 3;

fn main() -> Result<()> {
    terminal::install_panic_hook();

    let document = match std::env::args().nth(1) {
        Some(path) => Document::open(path)?,
        None => Document::default(),
    };

    let term = Terminal::enter()?;
    let result = run(&term, &mut Editor::new(document));
    drop(term);
    result
}

fn run(term: &Terminal, editor: &mut Editor) -> Result<()> {
    let mut out = io::stdout();

    loop {
        let (width, height) = crossterm::terminal::size()?;
        editor.view.fit(
            width as usize,
            height as usize,
            editor.document.text.line_count(),
        );
        render::draw(
            &mut out,
            &editor.document,
            &editor.view,
            &status(term, editor),
        )?;

        match event::read()? {
            Event::Key(event) => match Key::from_event(&event) {
                Some(key) if is_quit(key) => return Ok(()),
                Some(key) => handle_key(key, &event, editor),
                None => {}
            },
            Event::Mouse(mouse) => handle_mouse(mouse, editor),
            _ => {}
        }
    }
}

fn is_quit(key: Key) -> bool {
    key.code == KeyCode::Char('q') && key.mods == Mods::CTRL
}

/// Temporary bindings: these become commands once the keymap exists.
fn handle_key(key: Key, event: &KeyEvent, editor: &mut Editor) {
    use Direction::{Backward, Forward};

    let extend = key.mods.contains(Mods::SHIFT);
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    let page = editor.view.height.saturating_sub(1);

    match key.code {
        KeyCode::Backspace if ctrl => editor.delete(Motion::Word(Backward)),
        KeyCode::Delete if ctrl => editor.delete(Motion::Word(Forward)),
        KeyCode::Backspace => editor.delete(Motion::Grapheme(Backward)),
        KeyCode::Delete => editor.delete(Motion::Grapheme(Forward)),
        KeyCode::Enter => editor.insert("\n"),
        KeyCode::Tab => editor.insert("\t"),

        KeyCode::Char('z') if ctrl && extend => editor.redo(),
        KeyCode::Char('y') if ctrl => editor.redo(),
        KeyCode::Char('z') if ctrl => editor.undo(),
        KeyCode::Char('s') if ctrl => {
            let _ = editor.document.save();
        }

        KeyCode::Char('d') if ctrl => editor.select_next_occurrence(),
        KeyCode::Char('l') if ctrl && extend => editor.select_all_occurrences(),
        KeyCode::Up if ctrl && alt => editor.add_cursor(Backward),
        KeyCode::Down if ctrl && alt => editor.add_cursor(Forward),

        KeyCode::Left if ctrl => editor.move_selections(Motion::Word(Backward), extend),
        KeyCode::Right if ctrl => editor.move_selections(Motion::Word(Forward), extend),
        KeyCode::Left => editor.move_selections(Motion::Grapheme(Backward), extend),
        KeyCode::Right => editor.move_selections(Motion::Grapheme(Forward), extend),
        KeyCode::Up => editor.move_selections(Motion::Row(Backward), extend),
        KeyCode::Down => editor.move_selections(Motion::Row(Forward), extend),
        KeyCode::Home if ctrl => editor.move_selections(Motion::Document(Backward), extend),
        KeyCode::End if ctrl => editor.move_selections(Motion::Document(Forward), extend),
        KeyCode::Home => editor.move_selections(Motion::LineEdge(Backward), extend),
        KeyCode::End => editor.move_selections(Motion::LineEdge(Forward), extend),
        KeyCode::PageUp => {
            for _ in 0..page {
                editor.move_selections(Motion::Row(Backward), extend);
            }
        }
        KeyCode::PageDown => {
            for _ in 0..page {
                editor.move_selections(Motion::Row(Forward), extend);
            }
        }

        KeyCode::Esc => editor.view.selections.collapse_to_primary(),
        _ => {
            if let Some(character) = key::text_char(event) {
                insert_char(editor, character);
            }
        }
    }
}

fn insert_char(editor: &mut Editor, character: char) {
    let mut buffer = [0u8; 4];
    editor.insert(character.encode_utf8(&mut buffer));
}

fn handle_mouse(mouse: MouseEvent, editor: &mut Editor) {
    let view = &mut editor.view;
    let text = &editor.document.text;
    let column = (mouse.column as usize).saturating_sub(view.gutter);
    let row = mouse.row as usize;

    match mouse.kind {
        MouseEventKind::ScrollUp => view.scroll_lines(-MOUSE_SCROLL_LINES, text),
        MouseEventKind::ScrollDown => view.scroll_lines(MOUSE_SCROLL_LINES, text),
        MouseEventKind::Down(MouseButton::Left) if row < view.height => {
            let offset = view.offset_at_screen(text, row, column);
            match mouse.modifiers.contains(KeyModifiers::ALT) {
                true => view.selections.push(Selection::cursor(offset)),
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

fn status(term: &Terminal, editor: &Editor) -> String {
    let document = &editor.document;
    let path = document
        .path
        .as_ref()
        .map_or("[scratch]".to_owned(), |path| path.display().to_string());
    let path = match document.modified {
        true => format!("{path} •"),
        false => path,
    };
    let keyboard = match term.keyboard_enhanced {
        true => "enhanced",
        false => "legacy (some keys unavailable)",
    };

    let head = editor.view.selections.primary().head;
    let line = document.text.line_of(head);
    let column = head - document.text.line_start(line);
    let cursors = match editor.view.selections.len() {
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
