mod clipboard;
mod document;
mod editor;
mod key;
mod language;
mod layout;
mod fuzzy;
mod indent;
mod movement;
mod panel;
mod picker;
mod prompt;
mod render;
mod selection;
mod terminal;
mod text;
mod transaction;
mod view;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::document::Document;
use crate::editor::Editor;
use crate::key::{Key, KeyCode, Mods};
use crate::movement::{Direction, Motion};
use crate::panel::Panel;
use crate::picker::Pick;
use crate::prompt::Purpose;
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
    let mut editor = Editor::new(document).with_system_clipboard();
    if !term.keyboard_enhanced {
        editor.message = Some("Legacy keyboard: some combinations are unavailable".to_owned());
    }

    let result = run(&mut editor);
    drop(term);
    result
}

fn run(editor: &mut Editor) -> Result<()> {
    let mut out = io::stdout();

    loop {
        let (width, height) = crossterm::terminal::size()?;
        editor.view.fit(
            width as usize,
            height as usize,
            editor.document.text.line_count(),
            editor.panel_height(),
        );

        let (left, right) = status(editor);
        render::draw(&mut out, editor, &left, &right)?;

        // A panel still reading its candidates must not let the loop go back
        // to sleep, or the list would stop filling until the next keystroke.
        if editor.is_loading() && !event::poll(Duration::ZERO)? {
            editor.load_more();
            continue;
        }

        handle(event::read()?, editor);
        // Whatever else already arrived is processed before drawing again, so
        // a burst of input costs one frame rather than one frame per event.
        while !editor.quit && event::poll(Duration::ZERO)? {
            handle(event::read()?, editor);
        }

        if editor.quit {
            return Ok(());
        }
    }
}

fn handle(event: Event, editor: &mut Editor) {
    match event {
        Event::Key(event) => match Key::from_event(&event) {
            // An open panel owns the keyboard (ADR 12), so even quitting waits
            // for it to be dismissed rather than throwing it away.
            Some(key) if is_quit(key) && editor.panel.is_none() => editor.request_quit(),
            Some(key) => {
                editor.message = None;
                match editor.panel.is_some() {
                    true => handle_panel_key(key, &event, editor),
                    false => handle_key(key, &event, editor),
                }
            }
            None => {}
        },
        Event::Mouse(mouse) => handle_mouse(mouse, editor),
        // Bracketed paste: the terminal hands over the whole text at once
        // instead of replaying it as keystrokes.
        Event::Paste(text) => editor.insert(&text),
        _ => {}
    }
}

fn is_quit(key: Key) -> bool {
    key.code == KeyCode::Char('q') && key.mods == Mods::CTRL
}

/// An open panel owns the keyboard: this is the context of ADR 8, written by
/// hand until the keymap exists.
fn handle_panel_key(key: Key, event: &KeyEvent, editor: &mut Editor) {
    match editor.panel {
        Some(Panel::Picker(_)) => handle_picker_key(key, event, editor),
        Some(Panel::Confirm(_)) => handle_confirm_key(key, editor),
        _ => handle_prompt_key(key, event, editor),
    }
}

fn handle_confirm_key(key: Key, editor: &mut Editor) {
    match key.code {
        KeyCode::Esc => editor.close_panel(),
        // Combinations are not answers: Ctrl+Q must not discard the changes it
        // is asking about just because it carries a `q`.
        KeyCode::Char(answer) if !key.mods.intersects(Mods::CTRL | Mods::ALT) => {
            editor.answer(answer)
        }
        _ => {}
    }
}

fn handle_picker_key(key: Key, event: &KeyEvent, editor: &mut Editor) {
    use Direction::{Backward, Forward};

    let ctrl = key.mods.contains(Mods::CTRL);
    let extend = key.mods.contains(Mods::SHIFT);
    let page = crate::picker::VISIBLE as isize;
    let Some(picker) = editor.panel.as_mut().and_then(Panel::as_picker_mut) else {
        return;
    };

    let filtered = match key.code {
        KeyCode::Esc => return editor.close_panel(),
        KeyCode::Enter => return editor.accept_pick(),
        // The list grows upwards from the query, so moving up the screen
        // means moving away from the best match.
        KeyCode::Up => {
            picker.move_selection(1);
            false
        }
        KeyCode::Down => {
            picker.move_selection(-1);
            false
        }
        KeyCode::PageUp => {
            picker.move_selection(page);
            false
        }
        KeyCode::PageDown => {
            picker.move_selection(-page);
            false
        }
        KeyCode::Backspace if ctrl => {
            picker.query.delete(Motion::Word(Backward));
            true
        }
        KeyCode::Backspace => {
            picker.query.delete(Motion::Grapheme(Backward));
            true
        }
        KeyCode::Delete => {
            picker.query.delete(Motion::Grapheme(Forward));
            true
        }
        KeyCode::Left if ctrl => {
            picker.query.move_cursor(Motion::Word(Backward), extend);
            false
        }
        KeyCode::Right if ctrl => {
            picker.query.move_cursor(Motion::Word(Forward), extend);
            false
        }
        KeyCode::Left => {
            picker.query.move_cursor(Motion::Grapheme(Backward), extend);
            false
        }
        KeyCode::Right => {
            picker.query.move_cursor(Motion::Grapheme(Forward), extend);
            false
        }
        KeyCode::Home => {
            picker.query.move_cursor(Motion::LineEdge(Backward), extend);
            false
        }
        KeyCode::End => {
            picker.query.move_cursor(Motion::LineEdge(Forward), extend);
            false
        }
        _ => match key::text_char(event) {
            Some(character) => {
                picker.query.insert(&character.to_string());
                true
            }
            None => false,
        },
    };

    if filtered {
        picker.refilter();
    }
}

fn handle_prompt_key(key: Key, event: &KeyEvent, editor: &mut Editor) {
    use Direction::{Backward, Forward};

    let ctrl = key.mods.contains(Mods::CTRL);
    let extend = key.mods.contains(Mods::SHIFT);
    let Some(prompt) = editor.panel.as_mut().and_then(Panel::as_prompt_mut) else {
        return;
    };

    // Only an edited query needs the search run again; anything else would
    // drag the cursor back to the first hit.
    let edited = match key.code {
        KeyCode::Esc => return editor.close_panel(),
        KeyCode::Enter => return editor.submit_prompt(extend),
        KeyCode::F(3) => return editor.go_to_match(direction(extend)),
        KeyCode::Char('l') if ctrl && extend => return editor.select_all_matches(),
        KeyCode::Tab => {
            prompt.focus_next_field();
            false
        }
        KeyCode::Backspace if ctrl => {
            prompt.delete(Motion::Word(Backward));
            true
        }
        KeyCode::Delete if ctrl => {
            prompt.delete(Motion::Word(Forward));
            true
        }
        KeyCode::Backspace => {
            prompt.delete(Motion::Grapheme(Backward));
            true
        }
        KeyCode::Delete => {
            prompt.delete(Motion::Grapheme(Forward));
            true
        }
        KeyCode::Char('a') if ctrl => {
            prompt.select_all();
            false
        }
        KeyCode::Left if ctrl => {
            prompt.move_cursor(Motion::Word(Backward), extend);
            false
        }
        KeyCode::Right if ctrl => {
            prompt.move_cursor(Motion::Word(Forward), extend);
            false
        }
        KeyCode::Left => {
            prompt.move_cursor(Motion::Grapheme(Backward), extend);
            false
        }
        KeyCode::Right => {
            prompt.move_cursor(Motion::Grapheme(Forward), extend);
            false
        }
        KeyCode::Home => {
            prompt.move_cursor(Motion::LineEdge(Backward), extend);
            false
        }
        KeyCode::End => {
            prompt.move_cursor(Motion::LineEdge(Forward), extend);
            false
        }
        _ => match key::text_char(event) {
            Some(character) => {
                prompt.insert(&character.to_string());
                true
            }
            None => false,
        },
    };

    if edited {
        editor.prompt_changed();
    }
}

fn direction(backwards: bool) -> Direction {
    match backwards {
        true => Direction::Backward,
        false => Direction::Forward,
    }
}

/// Temporary bindings: these become commands once the keymap exists.
fn handle_key(key: Key, event: &KeyEvent, editor: &mut Editor) {
    use Direction::{Backward, Forward};

    let extend = key.mods.contains(Mods::SHIFT);
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);

    match key.code {
        KeyCode::Backspace if ctrl => editor.delete(Motion::Word(Backward)),
        KeyCode::Delete if ctrl => editor.delete(Motion::Word(Forward)),
        KeyCode::Backspace => editor.delete(Motion::Grapheme(Backward)),
        KeyCode::Delete => editor.delete(Motion::Grapheme(Forward)),
        KeyCode::Enter => editor.insert_newline(),
        KeyCode::Tab if extend => editor.outdent(),
        KeyCode::Tab => editor.indent(),

        KeyCode::Char('z') if ctrl && extend => editor.redo(),
        KeyCode::Char('y') if ctrl => editor.redo(),
        KeyCode::Char('z') if ctrl => editor.undo(),
        KeyCode::Char('s') if ctrl => {
            editor.save();
        }
        KeyCode::Char('g') if ctrl => editor.open_prompt(Purpose::GotoLine),
        KeyCode::Char('f') if ctrl => editor.open_prompt(Purpose::Search),
        KeyCode::Char('h') if ctrl => editor.open_prompt(Purpose::Replace),
        // Temporary: this belongs in the command palette.
        KeyCode::Char('m') if ctrl && extend => editor.open_picker(Pick::Language),
        KeyCode::F(3) if extend => editor.search_next(Backward),
        KeyCode::F(3) => editor.search_next(Forward),

        KeyCode::Char('c') if ctrl => editor.copy(),
        KeyCode::Char('x') if ctrl => editor.cut(),
        KeyCode::Char('v') if ctrl => editor.paste(),
        KeyCode::Char('d') if ctrl => editor.select_next_occurrence(),
        KeyCode::Char('l') if ctrl && extend => editor.select_all_occurrences(),
        KeyCode::Char('l') if ctrl => editor.select_line(),
        KeyCode::Char('a') if ctrl => editor.select_all(),
        KeyCode::Char('/') if ctrl => editor.toggle_comment(),
        KeyCode::Up if ctrl && alt => editor.add_cursor(Backward),
        KeyCode::Down if ctrl && alt => editor.add_cursor(Forward),
        KeyCode::Up if ctrl => editor.scroll(-1),
        KeyCode::Down if ctrl => editor.scroll(1),

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
        KeyCode::PageUp => editor.move_selections(Motion::Page(Backward), extend),
        KeyCode::PageDown => editor.move_selections(Motion::Page(Forward), extend),

        KeyCode::Esc if editor.view.selections.len() > 1 => editor.remove_secondary_cursors(),
        KeyCode::Esc => editor.cancel_selection(),
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
    let column = (mouse.column as usize).saturating_sub(view.margin + view.gutter);
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

/// The right side is the document, always in the same place. The left side is
/// whatever is going on right now, so the eye only goes there when something
/// is happening.
fn status(editor: &Editor) -> (String, String) {
    let document = &editor.document;
    let line = document.text.line_of(editor.view.selections.primary().head);
    let column = editor.cursor_column();

    let path = document
        .path
        .as_ref()
        .map_or("[scratch]".to_owned(), |path| path.display().to_string());
    let right = format!(
        "{path}{} · {} · {} lines · {}:{}",
        if document.is_modified() { "*" } else { "" },
        document.language.name,
        document.text.line_count(),
        line + 1,
        column + 1,
    );

    let left = match &editor.message {
        Some(message) => message.clone(),
        None => {
            let cursors = editor.view.selections.len();
            [
                editor.panel_status(),
                match cursors {
                    1 => String::new(),
                    count => format!("{count} cursors"),
                },
            ]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
        }
    };

    (format!(" {left}"), right)
}
