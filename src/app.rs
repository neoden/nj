//! The loop and the dispatch around the editor: what the process does with
//! events, as opposed to what the editor does with commands.

use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::editor::Editor;
use crate::key::{Key, KeyCode, Keystroke, Mods};
use crate::movement::{Direction, Motion};
use crate::panel::Outcome;
use crate::picker::Pick;
use crate::prompt::Purpose;
use crate::render;
use crate::selection::{Selection, SelectionSet};

const MOUSE_SCROLL_LINES: isize = 3;

pub fn run(editor: &mut Editor, out: &mut impl Write) -> Result<()> {
    loop {
        let (width, height) = crossterm::terminal::size()?;
        editor.view.fit(
            width as usize,
            height as usize,
            editor.buffer.text.line_count(),
            editor.panel_height(),
        );

        let (left, right) = status(editor);
        render::draw(out, editor, &left, &right)?;

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

/// Public so that a test can drive the editor with events and never open a
/// terminal. This is the largest surface the suite does not reach yet.
pub fn handle(event: Event, editor: &mut Editor) {
    match event {
        Event::Key(event) => match Keystroke::from_event(&event) {
            // An open panel owns the keyboard (ADR 12), so even quitting waits
            // for it to be dismissed rather than throwing it away.
            Some(stroke) if is_quit(stroke.key) && editor.panel.is_none() => {
                editor.request_quit()
            }
            Some(stroke) => {
                editor.message = None;
                match editor.panel.is_some() {
                    true => handle_panel_key(stroke, editor),
                    false => handle_key(stroke, editor),
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

/// The panel decides what a key means; this only carries out what it asked
/// for. Every outcome is one method, so nothing here knows what kind of panel
/// is open.
fn handle_panel_key(stroke: Keystroke, editor: &mut Editor) {
    let Some(panel) = editor.panel.as_mut() else {
        return;
    };

    match panel.handle_key(stroke) {
        Outcome::Nothing => {}
        Outcome::Close => editor.close_panel(),
        Outcome::Changed => editor.prompt_changed(),
        Outcome::Pick => editor.accept_pick(),
        Outcome::Submit { backwards } => editor.submit_prompt(backwards),
        Outcome::Answer(answer) => editor.answer(answer),
        Outcome::GoToMatch(direction) => editor.go_to_match(direction),
        Outcome::SelectAllMatches => editor.select_all_matches(),
    }
}

/// Temporary bindings: these become commands once the keymap exists.
fn handle_key(stroke: Keystroke, editor: &mut Editor) {
    use Direction::{Backward, Forward};

    let key = stroke.key;
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
            if let Some(character) = stroke.text {
                editor.insert(character.encode_utf8(&mut [0; 4]));
            }
        }
    }
}

fn handle_mouse(mouse: MouseEvent, editor: &mut Editor) {
    let view = &mut editor.view;
    let text = &editor.buffer.text;
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
    let buffer = &editor.buffer;
    let line = buffer.text.line_of(editor.view.selections.primary().head);
    let column = editor.cursor_column();

    let right = format!(
        "{}{} · {} · {} lines · {}:{}",
        buffer.name(),
        if buffer.is_modified() { "*" } else { "" },
        buffer.language.name,
        buffer.text.line_count(),
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
