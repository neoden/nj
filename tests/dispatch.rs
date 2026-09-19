//! Dispatch had no tests at all while it lived in `main.rs` (H2): the largest
//! unchecked surface in the crate. From here it needs no terminal — an event
//! goes in, the editor is asked what came of it.

use std::ops::Range;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use ned::app;
use ned::buffer::Buffer;
use ned::editor::Editor;
use ned::panel::Panel;

fn editor() -> Editor {
    let mut editor = Editor::new(Buffer::default());
    editor.resize(80, 24);
    editor
}

/// Pasted rather than typed, so it lands as one edit with no auto-indent in
/// the way, and the cursor is put back at the start.
fn editor_with(content: &str) -> Editor {
    let mut editor = editor();
    app::handle(Event::Paste(content.to_owned()), &mut editor);
    press(&mut editor, KeyCode::Home, KeyModifiers::CONTROL);
    editor
}

fn press(editor: &mut Editor, code: KeyCode, mods: KeyModifiers) {
    app::handle(Event::Key(KeyEvent::new(code, mods)), editor);
}

fn mouse(editor: &mut Editor, kind: MouseEventKind, column: u16, row: u16, mods: KeyModifiers) {
    app::handle(
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: mods,
        }),
        editor,
    );
}

fn ranges(editor: &Editor) -> Vec<Range<usize>> {
    editor.view().selections.iter().map(|s| s.range()).collect()
}

fn head(editor: &Editor) -> usize {
    editor.view().selections.primary().head
}

fn typed(editor: &mut Editor, text: &str) {
    for character in text.chars() {
        press(editor, KeyCode::Char(character), KeyModifiers::NONE);
    }
}

fn content(editor: &Editor) -> String {
    editor.buffer().text().to_string()
}

fn ctrl(editor: &mut Editor, character: char) {
    press(editor, KeyCode::Char(character), KeyModifiers::CONTROL);
}

fn query(editor: &Editor) -> String {
    match editor.panel() {
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
    assert!(editor.should_quit());
}

#[test]
fn ctrl_q_on_a_dirty_buffer_asks_first() {
    let mut editor = editor();
    typed(&mut editor, "x");
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);

    assert!(!editor.should_quit());
    assert!(editor.panel().is_some());
}

/// D7. An open panel owns the keyboard (ADR 12), so `Ctrl+Q` is not an answer
/// to the question the panel is asking just because it carries a `q`.
#[test]
fn ctrl_q_inside_a_panel_discards_nothing() {
    let mut editor = editor();
    typed(&mut editor, "x");
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::CONTROL);

    assert!(!editor.should_quit());
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
    assert!(editor.panel().is_some());

    press(&mut editor, KeyCode::Esc, KeyModifiers::NONE);
    assert!(editor.panel().is_none());
    assert_eq!(content(&editor), "needle");
}

#[test]
fn enter_in_a_picker_takes_the_choice() {
    let mut editor = editor();
    open_language_picker(&mut editor);
    typed(&mut editor, "rust");
    press(&mut editor, KeyCode::Enter, KeyModifiers::NONE);

    assert!(editor.panel().is_none());
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

// What follows pins the decisions the dispatch makes by the order of its
// arms — the ones a keymap table has to reproduce with context predicates.

/// `Esc` sheds the extra cursors first, and only then the selection; the
/// primary keeps what it had selected across the first press.
#[test]
fn esc_drops_extra_cursors_before_it_drops_the_selection() {
    let mut editor = editor_with("ab\ncd");
    press(
        &mut editor,
        KeyCode::Down,
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    press(&mut editor, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(ranges(&editor), [0..1, 3..4]);

    press(&mut editor, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [3..4]);

    press(&mut editor, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [4..4]);
}

/// `Tab` reaches the next stop from a bare cursor and indents whole lines
/// under a selection; `Shift+Tab` arrives as `BackTab` and only ever outdents.
#[test]
fn tab_indents_by_what_is_selected_and_back_tab_outdents() {
    let mut editor = editor_with("ab");
    press(&mut editor, KeyCode::Right, KeyModifiers::NONE);
    press(&mut editor, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(content(&editor), "a   b");

    ctrl(&mut editor, 'a');
    press(&mut editor, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(content(&editor), "    a   b");

    press(&mut editor, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(content(&editor), "a   b");
}

/// In a prompt, `Tab` moves between fields instead of indenting anything.
#[test]
fn tab_in_a_prompt_moves_to_the_next_field() {
    let mut editor = editor_with("x");
    ctrl(&mut editor, 'h');
    typed(&mut editor, "x");
    press(&mut editor, KeyCode::Tab, KeyModifiers::NONE);
    typed(&mut editor, "y");

    let prompt = editor.prompt().expect("the replace prompt is open");
    assert_eq!((prompt.contents(0), prompt.contents(1)), ("x".into(), "y".into()));
    assert_eq!(content(&editor), "x");
}

/// `Enter` walks the hits forwards and `Shift+Enter` backwards, wrapping
/// both ways.
#[test]
fn enter_in_a_search_walks_the_matches_in_both_directions() {
    let mut editor = editor_with("a a a");
    ctrl(&mut editor, 'f');
    typed(&mut editor, "a");
    assert_eq!(ranges(&editor), [0..1]);

    press(&mut editor, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [2..3]);

    press(&mut editor, KeyCode::Enter, KeyModifiers::SHIFT);
    assert_eq!(ranges(&editor), [0..1]);
    press(&mut editor, KeyCode::Enter, KeyModifiers::SHIFT);
    assert_eq!(ranges(&editor), [4..5]);
}

/// `F3` means the same thing whether the prompt is open or closed: the next
/// hit of the last search.
#[test]
fn f3_walks_the_matches_inside_the_prompt_and_after_it_closes() {
    let mut editor = editor_with("a a a");
    ctrl(&mut editor, 'f');
    typed(&mut editor, "a");
    press(&mut editor, KeyCode::F(3), KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [2..3]);

    press(&mut editor, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut editor, KeyCode::F(3), KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [4..5]);
    press(&mut editor, KeyCode::F(3), KeyModifiers::SHIFT);
    assert_eq!(ranges(&editor), [2..3]);
}

/// `Ctrl+Shift+L` puts a cursor on every hit of the open query; the same
/// key with the prompt closed searches for the word under the cursor, and
/// so respects word boundaries. That the two differ is S2, not a decision —
/// but until it is resolved the table has to reproduce both.
#[test]
fn ctrl_shift_l_takes_every_match_of_the_query_or_of_the_word() {
    let mut editor = editor_with("a ab a");
    ctrl(&mut editor, 'f');
    typed(&mut editor, "a");
    press(
        &mut editor,
        KeyCode::Char('l'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert!(editor.panel().is_none());
    assert_eq!(ranges(&editor), [0..1, 2..3, 5..6]);

    // From a bare cursor the first press only uncovers the word, as `Ctrl+D`
    // does; the second takes every whole-word hit.
    press(&mut editor, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut editor, KeyCode::Home, KeyModifiers::CONTROL);
    for _ in 0..2 {
        press(
            &mut editor,
            KeyCode::Char('l'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
    }
    assert_eq!(ranges(&editor), [0..1, 5..6]);
}

#[test]
fn ctrl_l_selects_the_line() {
    let mut editor = editor_with("ab\ncd");
    ctrl(&mut editor, 'l');
    assert_eq!(ranges(&editor), [0..3]);
}

#[test]
fn redo_answers_to_ctrl_y_and_to_ctrl_shift_z() {
    let mut editor = editor();
    typed(&mut editor, "a");

    ctrl(&mut editor, 'z');
    assert_eq!(content(&editor), "");
    ctrl(&mut editor, 'y');
    assert_eq!(content(&editor), "a");

    ctrl(&mut editor, 'z');
    press(
        &mut editor,
        KeyCode::Char('z'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(content(&editor), "a");
}

/// The arrow keys with modifiers are three different commands: bare moves
/// the cursor, `Ctrl` scrolls the page under it, `Ctrl+Alt` adds a cursor.
#[test]
fn arrows_move_scroll_or_add_a_cursor_by_their_modifiers() {
    let mut editor = editor_with(&"line\n".repeat(100));

    press(&mut editor, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(head(&editor), 5);
    assert_eq!(editor.view().top_line, 0);

    press(&mut editor, KeyCode::Down, KeyModifiers::CONTROL);
    assert_eq!(head(&editor), 5);
    assert_eq!(editor.view().top_line, 1);

    press(
        &mut editor,
        KeyCode::Down,
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    assert_eq!(ranges(&editor), [5..5, 10..10]);
}

#[test]
fn ctrl_backspace_takes_a_word_and_backspace_a_character() {
    let mut editor = editor_with("foo bar");
    press(&mut editor, KeyCode::End, KeyModifiers::NONE);

    press(&mut editor, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(content(&editor), "foo ba");
    press(&mut editor, KeyCode::Backspace, KeyModifiers::CONTROL);
    assert_eq!(content(&editor), "foo ");
}

#[test]
fn home_and_end_reach_the_line_or_with_ctrl_the_document() {
    let mut editor = editor_with("ab\ncd");
    press(&mut editor, KeyCode::End, KeyModifiers::CONTROL);
    assert_eq!(head(&editor), 5);
    press(&mut editor, KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(head(&editor), 3);
    press(&mut editor, KeyCode::End, KeyModifiers::NONE);
    assert_eq!(head(&editor), 5);
    press(&mut editor, KeyCode::Home, KeyModifiers::CONTROL);
    assert_eq!(head(&editor), 0);
}

/// A combination nothing is bound to does nothing at all — it must not fall
/// through to insertion.
#[test]
fn an_unbound_combination_inserts_nothing() {
    let mut editor = editor_with("x");
    ctrl(&mut editor, 'k');
    press(
        &mut editor,
        KeyCode::Char('k'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    press(&mut editor, KeyCode::Char('k'), KeyModifiers::ALT);

    assert_eq!(content(&editor), "x");
    assert!(editor.panel().is_none());
}

#[test]
fn enter_in_goto_line_jumps_and_closes_the_prompt() {
    let mut editor = editor_with("ab\ncd\nef");
    ctrl(&mut editor, 'g');
    typed(&mut editor, "3");
    press(&mut editor, KeyCode::Enter, KeyModifiers::NONE);

    assert!(editor.panel().is_none());
    assert_eq!(head(&editor), 6);
}

/// A question is answered by a bare letter only; the same letter with a
/// modifier is a combination, not an answer. Saving a buffer that has
/// nowhere to go fails, and a failed save cancels the quit.
#[test]
fn a_confirm_takes_bare_letters_only_and_a_failed_save_cancels_the_quit() {
    let mut editor = editor_with("x");
    ctrl(&mut editor, 'q');
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::ALT);
    assert!(!editor.should_quit());
    assert!(editor.panel().is_some());

    press(&mut editor, KeyCode::Char('s'), KeyModifiers::NONE);
    assert!(!editor.should_quit());
    assert!(editor.panel().is_none());
    assert!(editor.message().is_some());

    ctrl(&mut editor, 'q');
    press(&mut editor, KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(editor.should_quit());
}

/// The mouse speaks in screen cells: past the gutter, down from the view's
/// top. A click places the cursor, `Alt` adds one, a drag extends.
#[test]
fn clicks_land_past_the_gutter_and_drags_extend() {
    let mut editor = editor_with("hello\nworld");
    let gutter = editor.view().gutter as u16;
    let down = MouseEventKind::Down(MouseButton::Left);

    mouse(&mut editor, down, gutter + 3, 1, KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [9..9]);

    mouse(&mut editor, down, gutter + 1, 0, KeyModifiers::ALT);
    assert_eq!(ranges(&editor), [1..1, 9..9]);

    mouse(&mut editor, down, gutter, 0, KeyModifiers::NONE);
    let drag = MouseEventKind::Drag(MouseButton::Left);
    mouse(&mut editor, drag, gutter + 2, 1, KeyModifiers::NONE);
    assert_eq!(ranges(&editor), [0..8]);
}

#[test]
fn the_wheel_scrolls_without_moving_the_cursor() {
    let mut editor = editor_with(&"line\n".repeat(100));
    mouse(&mut editor, MouseEventKind::ScrollDown, 0, 0, KeyModifiers::NONE);
    assert_eq!(editor.view().top_line, 3);
    assert_eq!(head(&editor), 0);
}
