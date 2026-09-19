//! Dispatch had no tests at all while it lived in `main.rs` (H2): the largest
//! unchecked surface in the crate. From here it needs no terminal — an event
//! goes in, the editor is asked what came of it.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use ned::app;
use ned::buffer::Buffer;
use ned::editor::Editor;
use ned::panel::Panel;

fn editor() -> Editor {
    let mut editor = Editor::new(Buffer::default());
    editor.resize(80, 24);
    editor
}

fn press(editor: &mut Editor, code: KeyCode, mods: KeyModifiers) {
    app::handle(Event::Key(KeyEvent::new(code, mods)), editor);
}

fn typed(editor: &mut Editor, text: &str) {
    for character in text.chars() {
        press(editor, KeyCode::Char(character), KeyModifiers::NONE);
    }
}

fn content(editor: &Editor) -> String {
    editor.buffer().text.to_string()
}

fn ctrl(editor: &mut Editor, character: char) {
    press(editor, KeyCode::Char(character), KeyModifiers::CONTROL);
}

fn query(editor: &Editor) -> String {
    match &editor.panel {
        Some(Panel::Picker(picker)) => picker.query.contents(),
        _ => panic!("a picker is open"),
    }
}

fn open_language_picker(editor: &mut Editor) {
    press(
        editor,
        KeyCode::Char('m'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
}

#[test]
fn typing_reaches_the_buffer() {
    let mut editor = editor();
    typed(&mut editor, "hello");
    assert_eq!(content(&editor), "hello");
}

#[test]
fn ctrl_q_on_a_clean_buffer_quits() {
    let mut editor = editor();
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert!(editor.quit);
}

#[test]
fn ctrl_q_on_a_dirty_buffer_asks_first() {
    let mut editor = editor();
    typed(&mut editor, "x");
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);

    assert!(!editor.quit);
    assert!(editor.panel.is_some());
}

/// D7. An open panel owns the keyboard (ADR 12), so `Ctrl+Q` is not an answer
/// to the question the panel is asking just because it carries a `q`.
#[test]
fn ctrl_q_inside_a_panel_discards_nothing() {
    let mut editor = editor();
    typed(&mut editor, "x");
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);

    assert!(!editor.quit);
    assert_eq!(content(&editor), "x");
}

/// The terminal hands pasted text over whole, so it is one edit and one undo
/// takes all of it back.
#[test]
fn a_bracketed_paste_is_a_single_edit() {
    let mut editor = editor();
    app::handle(Event::Paste("two\nlines".to_owned()), &mut editor);
    assert_eq!(content(&editor), "two\nlines");

    press(&mut editor, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(content(&editor), "");
}

#[test]
fn esc_closes_a_panel_and_leaves_the_text_alone() {
    let mut editor = editor();
    typed(&mut editor, "needle");
    ctrl(&mut editor, 'f');
    assert!(editor.panel.is_some());

    press(&mut editor, KeyCode::Esc, KeyModifiers::NONE);
    assert!(editor.panel.is_none());
    assert_eq!(content(&editor), "needle");
}

#[test]
fn enter_in_a_picker_takes_the_choice() {
    let mut editor = editor();
    open_language_picker(&mut editor);
    typed(&mut editor, "rust");
    press(&mut editor, KeyCode::Enter, KeyModifiers::NONE);

    assert!(editor.panel.is_none());
    assert_eq!(editor.buffer().language.name, "Rust");
}

/// S3: the picker's query and a prompt's fields were near-identical copies
/// that differed only where nobody had decided they should. Sharing one
/// implementation gives the query `Ctrl+A`, which it never had.
#[test]
fn a_picker_query_takes_select_all() {
    let mut editor = editor();
    open_language_picker(&mut editor);
    typed(&mut editor, "rust");
    ctrl(&mut editor, 'a');
    typed(&mut editor, "go");

    assert_eq!(query(&editor), "go");
}

/// The other half of the same accident: the query never had `Ctrl+Delete`.
#[test]
fn a_picker_query_takes_delete_word() {
    let mut editor = editor();
    open_language_picker(&mut editor);
    typed(&mut editor, "type script");
    press(&mut editor, KeyCode::Home, KeyModifiers::NONE);
    press(&mut editor, KeyCode::Delete, KeyModifiers::CONTROL);

    assert_eq!(query(&editor), " script");
}

/// A layout the keymap knows nothing about still reaches the command: the
/// physical key is what is bound (ADR 7).
#[test]
fn a_cyrillic_layout_still_finds_the_command() {
    let mut editor = editor();
    typed(&mut editor, "abc");
    // `ф` sits where `a` does, so this is the Ctrl+A binding.
    press(&mut editor, KeyCode::Char('ф'), KeyModifiers::CONTROL);

    assert_eq!(editor.view().selections.primary().range(), 0..3);
}
