//! The editor's own tests: what each command does, driven through the
//! methods that implement it. What a key means is `tests/dispatch.rs`.

use super::*;
use crate::buffer::language::Indent;
use crate::text::movement::Direction;
use crate::text::selection::{Selection, SelectionSet};
use crate::text::Text;

fn editor(content: &str, offset: usize) -> Editor {
    let mut editor = Editor::new(Buffer::from_text(Text::from_str(content)));
    editor.resize(80, 25);
    select(&mut editor, SelectionSet::single(offset));
    editor
}

fn content(editor: &Editor) -> String {
    editor.buffer().text().to_string()
}

/// Starting a test from a known place is a command like any other.
fn select(editor: &mut Editor, selections: impl Into<SelectionSet>) {
    editor.execute(Command::Select(Select::Set(selections.into())));
}

fn select_range(editor: &mut Editor, anchor: usize, head: usize) {
    select(
        editor,
        Selection {
            anchor,
            head,
            goal_column: None,
        },
    );
}

fn add_cursor(editor: &mut Editor, offset: usize) {
    let mut selections = editor.view().selections.clone();
    selections.push(Selection::cursor(offset));
    select(editor, selections);
}

#[test]
fn typing_lands_after_the_cursor() {
    let mut editor = editor("ac", 1);
    editor.insert("b");

    assert_eq!(content(&editor), "abc");
    assert_eq!(editor.view().selections.primary().head, 2);
}

#[test]
fn typing_replaces_a_selection() {
    let mut editor = editor("hello world", 0);
    select_range(&mut editor, 0, 5);
    editor.insert("bye");

    assert_eq!(content(&editor), "bye world");
    assert!(editor.view().selections.primary().is_empty());
}

#[test]
fn backspace_eats_one_character_and_delete_the_next() {
    let mut editor = editor("абв", 4);
    editor.delete(Motion::Grapheme(Direction::Backward));
    assert_eq!(content(&editor), "ав");

    editor.delete(Motion::Grapheme(Direction::Forward));
    assert_eq!(content(&editor), "а");
}

#[test]
fn backspace_at_the_start_of_a_document_does_nothing() {
    let mut editor = editor("x", 0);
    editor.delete(Motion::Grapheme(Direction::Backward));

    assert_eq!(content(&editor), "x");
    assert!(!editor.buffer().is_modified());
}

#[test]
fn deleting_a_word_stops_at_its_boundary() {
    let mut editor = editor("foo bar.baz", 11);
    editor.delete(Motion::Word(Direction::Backward));
    assert_eq!(content(&editor), "foo bar.");

    editor.delete(Motion::Word(Direction::Backward));
    assert_eq!(content(&editor), "foo bar");
}

#[test]
fn every_cursor_types_at_once() {
    let mut editor = editor("a a a", 0);
    add_cursor(&mut editor, 2);
    add_cursor(&mut editor, 4);
    editor.insert("!");

    assert_eq!(content(&editor), "!a !a !a");
    assert_eq!(editor.view().selections.len(), 3);
}

#[test]
fn undo_and_redo_walk_the_history() {
    let mut editor = editor("start", 5);
    editor.insert(" one");
    editor.insert(" two");
    assert_eq!(content(&editor), "start one two");

    editor.undo();
    assert_eq!(content(&editor), "start one");
    editor.undo();
    assert_eq!(content(&editor), "start");
    editor.undo();
    assert_eq!(content(&editor), "start");

    editor.redo();
    editor.redo();
    assert_eq!(content(&editor), "start one two");
}

#[test]
fn undo_puts_the_cursors_back() {
    let mut editor = editor("a a", 1);
    add_cursor(&mut editor, 3);
    editor.insert("!");
    assert_eq!(content(&editor), "a! a!");

    editor.undo();
    let heads: Vec<_> = editor
        .view()
        .selections
        .iter()
        .map(|selection| selection.head)
        .collect();
    assert_eq!(heads, [1, 3]);
}

fn ranges(editor: &Editor) -> Vec<std::ops::Range<usize>> {
    editor
        .view()
        .selections
        .iter()
        .map(Selection::range)
        .collect()
}

#[test]
fn the_first_press_selects_the_word_under_the_cursor() {
    let mut editor = editor("foo bar foo", 1);
    editor.select_next_occurrence();

    assert_eq!(editor.view().selections.len(), 1);
    assert_eq!(editor.view().selections.primary().range(), 0..3);
}

#[test]
fn a_cursor_resting_after_a_word_still_finds_it() {
    let mut editor = editor("foo bar", 3);
    editor.select_next_occurrence();

    assert_eq!(editor.view().selections.primary().range(), 0..3);
}

#[test]
fn further_presses_add_occurrences_and_wrap() {
    let mut editor = editor("foo bar foo baz foo", 0);
    editor.select_next_occurrence();
    editor.select_next_occurrence();
    assert_eq!(ranges(&editor), [0..3, 8..11]);

    editor.select_next_occurrence();
    assert_eq!(ranges(&editor), [0..3, 8..11, 16..19]);

    // Everything is selected, so the wrap finds nothing new.
    editor.select_next_occurrence();
    assert_eq!(ranges(&editor), [0..3, 8..11, 16..19]);
}

#[test]
fn a_whole_word_selection_skips_words_containing_it() {
    let mut editor = editor("foo foobar foo", 0);
    editor.select_next_occurrence();
    editor.select_next_occurrence();

    assert_eq!(ranges(&editor), [0..3, 11..14]);
}

#[test]
fn a_word_selected_by_hand_is_taken_literally() {
    let mut editor = editor("foo foobar foo", 0);
    select_range(&mut editor, 0, 3);
    editor.select_next_occurrence();

    assert_eq!(ranges(&editor), [0..3, 4..7]);
}

#[test]
fn a_partial_selection_matches_anywhere() {
    let mut editor = editor("foo foobar", 0);
    select_range(&mut editor, 0, 2);
    editor.select_next_occurrence();

    assert_eq!(ranges(&editor), [0..2, 4..6]);
}

#[test]
fn selecting_all_occurrences_replaces_the_cursors() {
    let mut editor = editor("x = x + x", 0);
    editor.select_next_occurrence();
    editor.select_all_occurrences();

    assert_eq!(ranges(&editor), [0..1, 4..5, 8..9]);
}

#[test]
fn every_occurrence_can_be_edited_at_once() {
    let mut editor = editor("id, id, id", 0);
    editor.select_next_occurrence();
    editor.select_all_occurrences();
    editor.insert("key");

    assert_eq!(content(&editor), "key, key, key");
}

#[test]
fn cursors_stack_up_and_down() {
    let mut editor = editor("aaaa\nbbbb\ncccc", 6);
    editor.add_cursor(Direction::Forward);
    assert_eq!(ranges(&editor), [6..6, 11..11]);

    editor.add_cursor(Direction::Backward);
    assert_eq!(ranges(&editor), [1..1, 6..6, 11..11]);
}

#[test]
fn cursors_do_not_stack_past_the_document() {
    let mut editor = editor("only", 2);
    editor.add_cursor(Direction::Backward);
    editor.add_cursor(Direction::Forward);

    assert_eq!(editor.view().selections.len(), 1);
}

#[test]
fn a_stacked_cursor_keeps_the_goal_column_over_a_short_line() {
    let mut editor = editor("aaaa\nx\ncccc", 3);
    editor.add_cursor(Direction::Forward);
    editor.add_cursor(Direction::Forward);

    assert_eq!(ranges(&editor), [3..3, 6..6, 10..10]);
}

/// Goes through `execute` so that the focus routing is what puts the
/// text into the prompt and runs the search again.
fn type_into_prompt(editor: &mut Editor, typed: &str) {
    for character in typed.chars() {
        editor.execute(Command::Edit(Edit::Insert(character.to_string())));
    }
}

fn indented(content: &str, indent: Indent, offset: usize) -> Editor {
    let mut buffer = Buffer::from_text(Text::from_str(content));
    buffer.indent = indent;
    let mut editor = Editor::new(buffer);
    editor.resize(80, 25);
    select(&mut editor, SelectionSet::single(offset));
    editor
}

fn rust_editor(content: &str, offset: usize) -> Editor {
    let mut editor = editor(content, offset);
    editor.set_language(language::by_name("Rust").unwrap());
    editor
}

#[test]
fn commenting_and_uncommenting_a_line() {
    let mut editor = rust_editor("    code();\n", 4);
    editor.toggle_comment();
    assert_eq!(content(&editor), "    // code();\n");

    editor.toggle_comment();
    assert_eq!(content(&editor), "    code();\n");
}

#[test]
fn markers_line_up_with_the_shallowest_line() {
    let mut editor = rust_editor("  one();\n      two();\n", 0);
    select_range(&mut editor, 0, 20);
    editor.toggle_comment();

    assert_eq!(content(&editor), "  // one();\n  //     two();\n");
}

#[test]
fn a_half_commented_block_becomes_fully_commented() {
    let mut editor = rust_editor("// one();\ntwo();\n", 0);
    select_range(&mut editor, 0, 16);
    editor.toggle_comment();

    assert_eq!(content(&editor), "// // one();\n// two();\n");
}

#[test]
fn blank_lines_are_left_alone() {
    let mut editor = rust_editor("one();\n\ntwo();\n", 0);
    select_range(&mut editor, 0, 14);
    editor.toggle_comment();

    assert_eq!(content(&editor), "// one();\n\n// two();\n");
}

#[test]
fn a_language_without_line_comments_says_so() {
    let mut editor = editor("{}\n", 0);
    editor.set_language(language::by_name("JSON").unwrap());
    editor.toggle_comment();

    assert_eq!(content(&editor), "{}\n");
    assert!(editor.message.is_some());
}

#[test]
fn a_new_line_keeps_the_indentation() {
    let mut editor = editor("    deep\n", 8);
    editor.insert_newline();

    assert_eq!(content(&editor), "    deep\n    \n");
    assert_eq!(editor.view().selections.primary().head, 13);
}

#[test]
fn only_the_indentation_before_the_cursor_is_copied() {
    let mut editor = editor("      deep\n", 3);
    editor.insert_newline();

    // Three spaces were copied; the other three stayed with the text.
    assert_eq!(content(&editor), "   \n      deep\n");
}

#[test]
fn a_line_left_holding_only_indentation_is_emptied() {
    let mut editor = editor("    \n", 4);
    editor.insert_newline();

    assert_eq!(content(&editor), "\n    \n");
}

#[test]
fn every_cursor_keeps_its_own_indentation() {
    let mut editor = editor("  one\n    two\n", 5);
    add_cursor(&mut editor, 13);
    editor.insert_newline();

    assert_eq!(content(&editor), "  one\n  \n    two\n    \n");
}

#[test]
fn a_new_line_replaces_what_was_selected() {
    let mut editor = editor("    keep drop\n", 0);
    select_range(&mut editor, 9, 13);
    editor.insert_newline();

    assert_eq!(content(&editor), "    keep \n    \n");
}

#[test]
fn the_cursor_column_is_a_display_column() {
    // Three Cyrillic letters are six bytes but three columns.
    assert_eq!(editor("привет", 6).cursor_column(), 3);
    // A tab is one byte and a jump to the next stop.
    assert_eq!(editor("\tx", 2).cursor_column(), 5);
}

#[test]
fn tab_moves_to_the_next_stop_rather_than_a_whole_level() {
    let mut editor = indented("ab", Indent::Spaces(4), 2);
    editor.indent();
    assert_eq!(content(&editor), "ab  ");

    editor.indent();
    assert_eq!(content(&editor), "ab      ");
}

#[test]
fn tab_inserts_a_tab_when_the_file_uses_them() {
    let mut editor = indented("ab", Indent::Tabs, 2);
    editor.indent();
    assert_eq!(content(&editor), "ab\t");
}

#[test]
fn a_selection_across_lines_indents_them_all() {
    let mut editor = indented("one\ntwo\nthree\n", Indent::Spaces(2), 0);
    select_range(&mut editor, 1, 9);
    editor.indent();

    assert_eq!(content(&editor), "  one\n  two\n  three\n");
    // Snapped to the line start, so the new indentation is inside the
    // selection rather than hanging off its left edge.
    assert_eq!(editor.view().selections.primary().range(), 0..15);
}

#[test]
fn a_line_under_two_cursors_is_indented_once() {
    let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
    select_range(&mut editor, 0, 5);
    // A second cursor on a line the selection already covers.
    add_cursor(&mut editor, 6);
    editor.indent();

    assert_eq!(content(&editor), "  one\n  two\n");
}

#[test]
fn a_selection_ending_on_a_line_start_leaves_that_line_alone() {
    let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
    // What Ctrl+L produces: the line and the break after it.
    select_range(&mut editor, 0, 4);
    editor.indent();

    assert_eq!(content(&editor), "  one\ntwo\n");
}

#[test]
fn any_selection_at_all_means_indent_the_line() {
    let mut editor = indented("one two\n", Indent::Spaces(2), 0);
    select_range(&mut editor, 4, 7);
    editor.indent();

    // The selected word is indented with its line, not replaced by a tab.
    assert_eq!(content(&editor), "  one two\n");
}

#[test]
fn bare_cursors_get_a_tab_stop_each() {
    let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
    add_cursor(&mut editor, 4);
    editor.indent();

    assert_eq!(content(&editor), "  one\n  two\n");
}

#[test]
fn backspace_in_the_indentation_removes_a_whole_level() {
    let mut editor = indented("        code\n", Indent::Spaces(4), 8);
    editor.delete(Motion::Grapheme(Direction::Backward));

    assert_eq!(content(&editor), "    code\n");
}

#[test]
fn backspace_lands_on_the_previous_stop_not_a_fixed_step() {
    let mut editor = indented("      code\n", Indent::Spaces(4), 6);
    editor.delete(Motion::Grapheme(Direction::Backward));

    assert_eq!(content(&editor), "    code\n");
}

#[test]
fn backspace_after_text_removes_one_character() {
    let mut editor = indented("    a    b", Indent::Spaces(4), 9);
    editor.delete(Motion::Grapheme(Direction::Backward));

    assert_eq!(content(&editor), "    a   b");
}

#[test]
fn backspace_with_tabs_is_left_alone() {
    let mut editor = indented("\t\tcode\n", Indent::Tabs, 2);
    editor.delete(Motion::Grapheme(Direction::Backward));

    assert_eq!(content(&editor), "\tcode\n");
}

#[test]
fn outdent_removes_one_level_and_no_more() {
    let mut editor = indented("      deep\n", Indent::Spaces(2), 8);
    editor.outdent();
    assert_eq!(content(&editor), "    deep\n");

    editor.outdent();
    assert_eq!(content(&editor), "  deep\n");
}

#[test]
fn outdent_stops_at_the_left_margin() {
    let mut editor = indented(" x\n", Indent::Spaces(4), 2);
    editor.outdent();
    assert_eq!(content(&editor), "x\n");

    // Nothing left to remove, so nothing is recorded: one undo is enough.
    editor.outdent();
    assert_eq!(content(&editor), "x\n");

    editor.undo();
    assert_eq!(content(&editor), " x\n");
}

#[test]
fn indenting_is_one_undo_step() {
    let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
    select_range(&mut editor, 0, 7);
    editor.indent();
    assert_eq!(content(&editor), "  one\n  two\n");

    editor.undo();
    assert_eq!(content(&editor), "one\ntwo\n");
}

#[test]
fn undoing_back_to_what_was_saved_is_not_a_change() {
    let path = std::env::temp_dir().join(format!("ned-{}-modified", std::process::id()));
    std::fs::write(&path, "start").unwrap();

    let mut editor = Editor::new(Buffer::open(&path).unwrap());
    select(&mut editor, SelectionSet::single(5));
    assert!(!editor.buffer().is_modified());

    editor.insert("!");
    assert!(editor.buffer().is_modified());

    editor.save(After::Stay);
    assert!(!editor.buffer().is_modified());

    // Back to what the file holds, so there is nothing to warn about.
    editor.undo();
    assert!(editor.buffer().is_modified());
    editor.redo();
    assert!(!editor.buffer().is_modified());

    // And an edit undone past the save is a change again.
    editor.insert("?");
    assert!(editor.buffer().is_modified());
    editor.undo();
    assert!(!editor.buffer().is_modified());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn typing_after_a_save_starts_its_own_revision() {
    let path = std::env::temp_dir().join(format!("ned-{}-revision", std::process::id()));
    std::fs::write(&path, "").unwrap();

    let mut editor = Editor::new(Buffer::open(&path).unwrap());
    type_text(&mut editor, "wor");
    editor.save(After::Stay);

    // The same word carries on, but it must not join the group that was
    // written to disk.
    type_text(&mut editor, "d");
    assert!(editor.buffer().is_modified());

    let _ = std::fs::remove_file(&path);
}

/// ADR 20, the half that waits for nothing: a save never overwrites
/// another writer's work without asking.
#[test]
fn a_file_changed_elsewhere_is_not_overwritten_unasked() {
    let path = std::env::temp_dir().join(format!("ned-{}-elsewhere", std::process::id()));
    std::fs::write(&path, "mine\n").unwrap();

    let mut editor = Editor::new(Buffer::open(&path).unwrap());
    editor.insert("!");
    std::fs::write(&path, "theirs, and longer\n").unwrap();

    editor.execute(Command::File(File::Save));
    assert!(matches!(
        editor.panel,
        Some(Panel::Confirm(Confirm {
            about: Question::Overwrite(After::Stay),
            ..
        }))
    ));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs, and longer\n");
    assert!(editor.buffer().is_modified());

    editor.execute(Command::Panel(PanelCommand::Answer('o')));
    assert!(editor.panel.is_none());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "!mine\n");
    assert!(!editor.buffer().is_modified());
    assert!(!editor.quit);

    let _ = std::fs::remove_file(&path);
}

/// The watcher only says the stamp moved; whether that is another writer
/// or our own save is decided on arrival.
#[test]
fn a_report_of_a_changed_file_is_checked_before_it_is_believed() {
    let path = std::env::temp_dir().join(format!("ned-{}-watched", std::process::id()));
    std::fs::write(&path, "mine\n").unwrap();
    let mut editor = Editor::new(Buffer::open(&path).unwrap());
    let id = editor.workspace.focused_buffer();

    // Our own save moves the stamp, and the report that follows it says
    // nothing.
    editor.insert("!");
    editor.execute(Command::File(File::Save));
    editor.receive(Report::FileChanged(id));
    assert_eq!(editor.message(), None);

    std::fs::write(&path, "theirs, and longer\n").unwrap();
    editor.receive(Report::FileChanged(id));
    assert_eq!(editor.message(), Some("File changed on disk"));

    let _ = std::fs::remove_file(&path);
}

/// The picker takes candidates from its own worker only: a batch from a
/// picker that has since been closed is not the new one's.
#[test]
fn a_picker_ignores_candidates_meant_for_another() {
    fn picker(editor: &Editor) -> &Picker {
        match &editor.panel {
            Some(Panel::Picker(picker)) => picker,
            _ => panic!("a picker is open"),
        }
    }

    let mut editor = editor("", 0);
    editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
    let stale = picker(&editor).job;

    // Opened again: the first worker is cancelled, but a batch of its
    // may still be in the queue.
    editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
    editor.finish_jobs();
    assert_ne!(picker(&editor).job, stale);
    let total = picker(&editor).total();
    assert!(total > 0);

    editor.receive(Report::Candidates {
        job: stale,
        batch: vec![picker::Candidate::plain("Stray")],
    });
    assert_eq!(picker(&editor).total(), total);
}

/// "Save and quit" that runs into another writer asks, and the answer
/// still quits: the intention to leave is not lost with the question.
#[test]
fn save_and_quit_still_quits_after_agreeing_to_overwrite() {
    let path = std::env::temp_dir().join(format!("ned-{}-overwrite-quit", std::process::id()));
    std::fs::write(&path, "mine\n").unwrap();

    let mut editor = Editor::new(Buffer::open(&path).unwrap());
    editor.insert("!");
    std::fs::write(&path, "theirs, and longer\n").unwrap();

    editor.execute(Command::Quit);
    editor.execute(Command::Panel(PanelCommand::Answer('s')));
    assert!(!editor.quit);
    assert!(matches!(
        editor.panel,
        Some(Panel::Confirm(Confirm {
            about: Question::Overwrite(After::Quit),
            ..
        }))
    ));

    // Declining leaves everything as it was: not written, not quit.
    editor.execute(Command::Panel(PanelCommand::Close));
    assert!(!editor.quit);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs, and longer\n");

    editor.execute(Command::Quit);
    editor.execute(Command::Panel(PanelCommand::Answer('s')));
    editor.execute(Command::Panel(PanelCommand::Answer('o')));
    assert!(editor.quit);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "!mine\n");

    let _ = std::fs::remove_file(&path);
}

fn scratch_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ned-{}-{name}", std::process::id()))
}

fn confirm(editor: &Editor) -> Option<&Question> {
    match &editor.panel {
        Some(Panel::Confirm(confirm)) => Some(&confirm.about),
        _ => None,
    }
}

/// F4: another file takes the place of the one in focus, and everything
/// that referred to the old text goes with it.
#[test]
fn opening_replaces_the_buffer_and_starts_the_view_over() {
    let path = scratch_path("open");
    std::fs::write(&path, "one\ntwo\n").unwrap();

    let mut editor = editor("scratch text", 8);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "text");
    editor.submit_prompt(false);
    assert!(editor.search.is_some());

    editor.execute(Command::File(File::Open(path.clone())));
    assert_eq!(content(&editor), "one\ntwo\n");
    assert_eq!(editor.buffer().name(), path.display().to_string());
    assert_eq!(ranges(&editor), [0..0]);
    assert!(editor.search.is_none());
    assert!(editor.panel.is_none());
    // The view keeps its place on the screen: it can still be drawn into.
    assert!(editor.view().width() > 0);

    // The file is watched from now on, and opening another lets go of it.
    let id = editor.workspace.focused_buffer();
    let watcher = editor.watchers[&id];
    assert!(editor.jobs.is_live(watcher));
    editor.execute(Command::File(File::Open(scratch_path("open-other"))));
    assert!(!editor.jobs.is_live(watcher));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_path_with_no_file_opens_empty() {
    let path = scratch_path("open-new");
    let _ = std::fs::remove_file(&path);

    let mut editor = editor("old", 0);
    editor.execute(Command::File(File::Open(path.clone())));
    assert_eq!(content(&editor), "");
    assert_eq!(editor.buffer().name(), path.display().to_string());
    assert!(!path.exists());
}

/// Unsaved changes are asked about, and "save first" carries on to open
/// once the save has gone through.
#[test]
fn opening_over_unsaved_changes_asks_first() {
    let mine = scratch_path("open-mine");
    let theirs = scratch_path("open-theirs");
    std::fs::write(&mine, "mine\n").unwrap();
    std::fs::write(&theirs, "theirs\n").unwrap();

    let mut editor = Editor::new(Buffer::open(&mine).unwrap());
    editor.insert("!");
    editor.execute(Command::File(File::Open(theirs.clone())));
    assert_eq!(confirm(&editor), Some(&Question::Leave(After::Open(theirs.clone()))));
    assert_eq!(content(&editor), "!mine\n");

    // Declining leaves everything as it was.
    editor.execute(Command::Panel(PanelCommand::Close));
    assert_eq!(content(&editor), "!mine\n");
    assert!(editor.buffer().is_modified());

    editor.execute(Command::File(File::Open(theirs.clone())));
    editor.execute(Command::Panel(PanelCommand::Answer('s')));
    assert_eq!(std::fs::read_to_string(&mine).unwrap(), "!mine\n");
    assert_eq!(content(&editor), "theirs\n");

    // Or the changes are dropped on purpose.
    editor.insert("?");
    editor.execute(Command::File(File::Open(mine.clone())));
    editor.execute(Command::Panel(PanelCommand::Answer('d')));
    assert_eq!(content(&editor), "!mine\n");
    assert_eq!(std::fs::read_to_string(&theirs).unwrap(), "theirs\n");

    let _ = std::fs::remove_file(&mine);
    let _ = std::fs::remove_file(&theirs);
}

/// A key that is not one of the answers changes nothing, the question
/// included.
#[test]
fn a_question_ignores_keys_that_answer_nothing() {
    let mut editor = editor("x", 0);
    editor.insert("!");
    editor.execute(Command::Quit);
    editor.execute(Command::Panel(PanelCommand::Answer('d')));
    assert_eq!(confirm(&editor), Some(&Question::Leave(After::Quit)));
    assert!(!editor.quit);
}

#[test]
fn saving_as_names_a_scratch_buffer_and_asks_before_taking_a_file_that_exists() {
    let fresh = scratch_path("save-as-fresh");
    let taken = scratch_path("save-as-taken");
    let _ = std::fs::remove_file(&fresh);
    std::fs::write(&taken, "taken\n").unwrap();

    let mut editor = editor("", 0);
    editor.insert("text");
    editor.execute(Command::File(File::SaveAs(fresh.clone())));
    assert_eq!(std::fs::read_to_string(&fresh).unwrap(), "text");
    assert_eq!(editor.buffer().name(), fresh.display().to_string());
    assert!(!editor.buffer().is_modified());

    // Saving under the name it already has asks nothing.
    editor.insert("!");
    editor.execute(Command::File(File::SaveAs(fresh.clone())));
    assert!(editor.panel.is_none());
    assert_eq!(std::fs::read_to_string(&fresh).unwrap(), "text!");

    editor.execute(Command::File(File::SaveAs(taken.clone())));
    assert_eq!(confirm(&editor), Some(&Question::Replace(taken.clone(), After::Stay)));
    // Not renamed until agreed to.
    assert_eq!(editor.buffer().name(), fresh.display().to_string());
    editor.execute(Command::Panel(PanelCommand::Answer('o')));
    assert_eq!(std::fs::read_to_string(&taken).unwrap(), "text!");
    assert_eq!(editor.buffer().name(), taken.display().to_string());

    let _ = std::fs::remove_file(&fresh);
    let _ = std::fs::remove_file(&taken);
}

#[test]
fn a_pipeline_buffer_is_not_replaced_and_saves_a_copy() {
    let path = scratch_path("pipeline-open");
    let _ = std::fs::remove_file(&path);

    let mut editor = editor("in", 0);
    editor.buffer_mut().backing = Backing::Stdio {
        committed: editor.buffer().text().clone(),
    };

    editor.execute(Command::File(File::Open(path.clone())));
    assert_eq!(content(&editor), "in");
    assert!(editor.message().is_some());

    editor.execute(Command::File(File::SaveAs(path.clone())));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "in");
    assert_eq!(editor.buffer().name(), "[stdin]");
    assert!(editor.message().unwrap().starts_with("Copied to"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_unchanged_document_quits_at_once() {
    let mut editor = editor("text", 0);
    editor.request_quit();

    assert!(editor.quit);
    assert!(editor.panel.is_none());
}

#[test]
fn unsaved_changes_ask_first() {
    let mut editor = editor("text", 0);
    editor.insert("!");
    editor.request_quit();

    assert!(!editor.quit);
    assert!(matches!(editor.panel, Some(Panel::Confirm(_))));
}

#[test]
fn the_question_can_be_answered_or_dropped() {
    let mut editor = editor("text", 0);
    editor.insert("!");

    editor.request_quit();
    editor.close_panel();
    assert!(!editor.quit);

    editor.request_quit();
    editor.answer('q');
    assert!(editor.quit);
    assert!(editor.panel.is_none());
}

#[test]
fn the_answer_key_is_the_physical_one() {
    let mut editor = editor("text", 0);
    editor.insert("!");
    editor.request_quit();

    // `й` sits where `q` does on a US keyboard.
    editor.answer('й');
    assert!(editor.quit);
}

/// A scratch buffer has nowhere to be written to, so "save and quit" asks
/// for a name first and quits once the file is there.
#[test]
fn saving_a_nameless_buffer_asks_for_a_name_and_carries_on() {
    let path = scratch_path("named-on-the-way-out");
    let _ = std::fs::remove_file(&path);

    let mut editor = editor("text", 0);
    editor.insert("!");
    editor.request_quit();
    editor.answer('s');
    assert!(!editor.quit);
    assert_eq!(
        editor.prompt().map(|prompt| prompt.purpose.clone()),
        Some(Purpose::SaveAs(After::Quit))
    );

    // Nothing typed: nothing happens, and the intention is dropped with
    // the prompt.
    editor.submit_prompt(false);
    assert!(editor.panel.is_none());
    assert!(!editor.quit);
    assert!(editor.buffer().is_modified());

    editor.request_quit();
    editor.answer('s');
    type_into_prompt(&mut editor, &path.display().to_string());
    editor.submit_prompt(false);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "!text");
    assert!(editor.quit);

    let _ = std::fs::remove_file(&path);
}

/// A save that fails says why and goes no further.
#[test]
fn a_save_that_fails_cancels_the_quit() {
    let path = scratch_path("unwritable").join("below");
    let mut editor = editor("text", 0);
    editor.insert("!");
    editor.execute(Command::File(File::SaveAs(path)));
    editor.request_quit();
    editor.answer('s');

    assert!(!editor.quit);
    assert!(editor.message().unwrap().starts_with("cannot write"));
}

/// The prompts behind `Ctrl+O` and `Ctrl+Shift+S`: a path typed in becomes
/// the command, and "save as" starts from the name the file has.
#[test]
fn the_path_prompts_open_and_save_what_is_typed() {
    let first = scratch_path("prompt-first");
    let second = scratch_path("prompt-second");
    std::fs::write(&first, "first\n").unwrap();
    let _ = std::fs::remove_file(&second);

    let mut editor = editor("", 0);
    editor.execute(Command::Panel(PanelCommand::Prompt(Purpose::Open)));
    type_into_prompt(&mut editor, &format!(" {} ", first.display()));
    editor.submit_prompt(false);
    assert_eq!(content(&editor), "first\n");

    editor.execute(Command::Panel(PanelCommand::Prompt(Purpose::SaveAs(After::Stay))));
    assert_eq!(editor.prompt().unwrap().contents(0), first.display().to_string());
    editor.execute(Command::Select(Select::All));
    type_into_prompt(&mut editor, &second.display().to_string());
    editor.submit_prompt(false);
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "first\n");
    assert_eq!(editor.buffer().name(), second.display().to_string());

    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
}

#[test]
fn goto_line_moves_the_cursor_to_its_start() {
    let mut editor = editor("one\ntwo\nthree\n", 0);
    editor.open_prompt(Purpose::GotoLine);
    type_into_prompt(&mut editor, "3");
    editor.submit_prompt(false);

    assert_eq!(editor.view().selections.primary().head, 8);
    assert!(editor.panel.is_none());
}

#[test]
fn goto_line_complains_about_nonsense() {
    let mut editor = editor("one\ntwo\n", 0);
    editor.open_prompt(Purpose::GotoLine);
    type_into_prompt(&mut editor, "later");
    editor.submit_prompt(false);

    assert_eq!(editor.view().selections.primary().head, 0);
    assert!(editor.message.is_some());
}

#[test]
fn select_all_covers_the_document_and_stays_put() {
    let mut editor = editor(&"line\n".repeat(100), 0);
    editor.resize(80, 11);
    editor.move_selections(Motion::Document(Direction::Forward), false);
    let top = editor.view().top_line;

    editor.select_all();
    assert_eq!(
        editor.view().selections.primary().range(),
        0..editor.buffer().text().len()
    );
    assert_eq!(editor.view().top_line, top);
}

#[test]
fn scrolling_leaves_the_cursors_alone() {
    let mut editor = editor(&"line\n".repeat(100), 0);
    editor.resize(80, 11);

    editor.scroll(5);
    assert_eq!(editor.view().top_line, 5);
    assert_eq!(editor.view().selections.primary().head, 0);

    editor.scroll(-99);
    assert_eq!(editor.view().top_line, 0);
}

#[test]
fn opening_search_takes_the_selected_text() {
    let mut editor = editor("alpha beta gamma", 0);
    select_range(&mut editor, 6, 10);
    editor.open_prompt(Purpose::Search);

    let prompt = editor.prompt().unwrap();
    assert_eq!(prompt.contents(0), "beta");
    // Seeded and selected, so the next keystroke replaces it.
    assert_eq!(prompt.fields[0].selection(), 0..4);
    // The match under the cursor stays put rather than jumping ahead.
    assert_eq!(editor.view().selections.primary().range(), 6..10);
}

#[test]
fn a_selection_across_lines_does_not_seed_the_query() {
    let mut editor = editor("one\ntwo", 0);
    select_range(&mut editor, 0, 7);
    editor.open_prompt(Purpose::Search);

    assert_eq!(editor.prompt().unwrap().contents(0), "");
}

#[test]
fn search_follows_every_keystroke() {
    let mut editor = editor("alpha beta gamma", 0);
    editor.open_prompt(Purpose::Search);

    type_into_prompt(&mut editor, "b");
    assert_eq!(editor.view().selections.primary().range(), 6..7);

    type_into_prompt(&mut editor, "eta");
    assert_eq!(editor.view().selections.primary().range(), 6..10);
}

#[test]
fn search_starts_over_from_where_it_opened() {
    let mut editor = editor("aa ab", 0);
    editor.open_prompt(Purpose::Search);

    // Typing `a` then `b` must not chase the first match forward.
    type_into_prompt(&mut editor, "ab");
    assert_eq!(editor.view().selections.primary().range(), 3..5);
}

#[test]
fn escape_closes_the_search_where_it_left_off() {
    let mut editor = editor("alpha beta", 2);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "beta");
    editor.close_panel();

    assert_eq!(editor.view().selections.primary().range(), 6..10);
    assert!(editor.panel.is_none());
    assert!(editor.matches().is_empty());
}

#[test]
fn enter_walks_the_matches_and_wraps() {
    let mut editor = editor("x . x . x", 0);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "x");
    assert_eq!(editor.view().selections.primary().range(), 0..1);

    editor.submit_prompt(false);
    assert_eq!(editor.view().selections.primary().range(), 4..5);
    editor.submit_prompt(false);
    assert_eq!(editor.view().selections.primary().range(), 8..9);
    editor.submit_prompt(false);
    assert_eq!(editor.view().selections.primary().range(), 0..1);

    editor.submit_prompt(true);
    assert_eq!(editor.view().selections.primary().range(), 8..9);

    // Walking the hits keeps the panel open.
    assert!(editor.panel.is_some());
}

#[test]
fn every_hit_is_collected_and_counted() {
    let mut editor = editor("x . x . x", 0);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "x");

    assert_eq!(editor.matches(), [0..1, 4..5, 8..9]);
    assert_eq!(editor.panel_status(), "Found: 1/3");

    editor.submit_prompt(false);
    assert_eq!(editor.panel_status(), "Found: 2/3");
}

#[test]
fn every_hit_can_become_a_cursor() {
    let mut editor = editor("id, id, id", 0);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "id");
    editor.submit_prompt(false);

    editor.select_all_matches();
    assert_eq!(ranges(&editor), [0..2, 4..6, 8..10]);
    // The keyboard goes back to the text, which is the point.
    assert!(editor.panel.is_none());
    // The hit we were standing on stays the primary one.
    assert_eq!(editor.view().selections.primary().range(), 4..6);

    editor.insert("key");
    assert_eq!(content(&editor), "key, key, key");
}

/// P10: a cursor on each of a hundred thousand hits is not a selection
/// anyone can edit with, so neither command makes one.
#[test]
fn too_many_hits_are_refused_rather_than_partly_selected() {
    let mut over = editor(&"x ".repeat(MAX_CURSORS + 1), 0);

    over.select_next_occurrence();
    assert_eq!(ranges(&over), [0..1]);
    over.select_all_occurrences();
    assert_eq!(ranges(&over), [0..1]);
    assert!(over.message().unwrap().starts_with("More than"));

    over.open_prompt(Purpose::Search);
    type_into_prompt(&mut over, "x");
    over.submit_prompt(false);
    over.select_all_matches();
    assert_eq!(over.view().selections.len(), 1);
    assert!(over.panel.is_some());

    // One fewer and the command goes through.
    let mut under = editor(&"x ".repeat(MAX_CURSORS), 0);
    under.select_next_occurrence();
    under.select_all_occurrences();
    assert_eq!(under.view().selections.len(), MAX_CURSORS);
}

#[test]
fn narrowing_a_query_finds_what_scanning_would() {
    let mut editor = editor("ab abc abcd", 0);
    editor.open_prompt(Purpose::Search);

    type_into_prompt(&mut editor, "a");
    assert_eq!(editor.matches(), [0..1, 3..4, 7..8]);

    type_into_prompt(&mut editor, "b");
    assert_eq!(editor.matches(), [0..2, 3..5, 7..9]);

    type_into_prompt(&mut editor, "cd");
    assert_eq!(editor.matches(), vec![7..11]);

    // Backspace cannot narrow, so this one has to read the document again.
    editor.execute(Command::Edit(Edit::Delete(Motion::Grapheme(Direction::Backward))));
    assert_eq!(editor.matches(), [3..6, 7..10]);
}

#[test]
fn narrowing_does_not_cut_a_character_in_half() {
    let mut editor = editor("аб абв", 0);
    editor.open_prompt(Purpose::Search);

    type_into_prompt(&mut editor, "а");
    assert_eq!(editor.matches(), [0..2, 5..7]);

    type_into_prompt(&mut editor, "бв");
    assert_eq!(editor.matches(), vec![5..11]);
}

#[test]
fn a_query_with_no_hits_counts_zero() {
    let mut editor = editor("nothing here", 0);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "zzz");

    assert!(editor.matches().is_empty());
    assert_eq!(editor.panel_status(), "Found: none");
}

#[test]
fn f3_still_works_once_the_strip_is_closed() {
    let mut editor = editor("x . x", 0);
    editor.open_prompt(Purpose::Search);
    type_into_prompt(&mut editor, "x");
    editor.close_panel();

    editor.search_next(Direction::Forward);
    assert_eq!(editor.view().selections.primary().range(), 4..5);
}

#[test]
fn replace_changes_every_occurrence_in_one_step() {
    let mut editor = editor("one two one", 0);
    editor.open_prompt(Purpose::Replace);
    type_into_prompt(&mut editor, "one");
    editor.execute(Command::Panel(PanelCommand::NextField));
    type_into_prompt(&mut editor, "1");
    editor.submit_prompt(false);

    assert_eq!(content(&editor), "1 two 1");

    editor.undo();
    assert_eq!(content(&editor), "one two one");
}

#[test]
fn selecting_a_line_takes_its_break_too() {
    let mut editor = editor("first\nsecond\nthird\n", 8);
    editor.select_line();

    assert_eq!(editor.view().selections.primary().range(), 6..13);
}

#[test]
fn selecting_again_swallows_the_next_line() {
    let mut editor = editor("first\nsecond\nthird\n", 8);
    editor.select_line();
    editor.select_line();

    assert_eq!(editor.view().selections.primary().range(), 6..19);
}

#[test]
fn a_selection_across_lines_expands_to_all_of_them() {
    let mut editor = editor("first\nsecond\nthird\n", 0);
    select_range(&mut editor, 3, 8);
    editor.select_line();

    assert_eq!(editor.view().selections.primary().range(), 0..13);
}

#[test]
fn selecting_the_last_line_stops_at_the_end() {
    let mut editor = editor("first\nlast", 7);
    editor.select_line();
    assert_eq!(editor.view().selections.primary().range(), 6..10);

    editor.select_line();
    assert_eq!(editor.view().selections.primary().range(), 6..10);
}

#[test]
fn selected_lines_can_be_cut_at_every_cursor() {
    let mut editor = editor("one\ntwo\nthree\n", 0);
    add_cursor(&mut editor, 8);
    editor.select_line();
    editor.cut();

    assert_eq!(content(&editor), "two\n");
}

#[test]
fn copying_and_pasting_a_selection() {
    let mut editor = editor("hello world", 0);
    editor.select_next_occurrence();
    editor.copy();

    select(&mut editor, SelectionSet::single(11));
    editor.paste();
    assert_eq!(content(&editor), "hello worldhello");
}

#[test]
fn pasting_replaces_a_selection() {
    let mut editor = editor("keep drop", 5);
    editor.select_next_occurrence();
    editor.copy();
    editor.paste();

    assert_eq!(content(&editor), "keep drop");
    assert_eq!(editor.view().selections.primary().head, 9);
}

#[test]
fn cutting_removes_what_it_took() {
    let mut editor = editor("hello world", 6);
    editor.select_next_occurrence();
    editor.cut();

    assert_eq!(content(&editor), "hello ");
    assert_eq!(editor.clipboard.text(), "world");
}

#[test]
fn copying_without_a_selection_takes_the_whole_line() {
    let mut editor = editor("first\nsecond\n", 8);
    editor.copy();

    assert_eq!(editor.clipboard.text(), "second\n");
    assert!(editor.clipboard.holds_lines());
}

#[test]
fn pasting_a_line_puts_it_above_and_keeps_the_column() {
    let mut editor = editor("first\nsecond\n", 8);
    editor.copy();
    editor.paste();

    assert_eq!(content(&editor), "first\nsecond\nsecond\n");
    // The cursor stayed on the same character of its own line.
    assert_eq!(editor.view().selections.primary().head, 15);
}

#[test]
fn a_last_line_without_a_break_still_copies_as_a_line() {
    let mut editor = editor("only", 2);
    editor.copy();

    assert_eq!(editor.clipboard.text(), "only\n");
}

#[test]
fn cutting_a_line_removes_it_whole() {
    let mut editor = editor("first\nsecond\nthird\n", 8);
    editor.cut();

    assert_eq!(content(&editor), "first\nthird\n");
}

#[test]
fn a_line_under_two_cursors_is_cut_once() {
    let mut editor = editor("one\ntwo\nthree\n", 0);
    add_cursor(&mut editor, 2);
    editor.cut();

    assert_eq!(content(&editor), "two\nthree\n");
    assert_eq!(editor.clipboard.text(), "one\n");
}

#[test]
fn a_line_under_two_cursors_is_pasted_once() {
    let mut editor = editor("one\ntwo\n", 0);
    editor.copy();
    add_cursor(&mut editor, 2);
    editor.paste();

    assert_eq!(content(&editor), "one\none\ntwo\n");
}

#[test]
fn cursors_on_separate_lines_still_get_a_line_each() {
    let mut editor = editor("one\ntwo\nthree\n", 0);
    add_cursor(&mut editor, 4);
    editor.cut();

    assert_eq!(content(&editor), "three\n");
    assert_eq!(editor.clipboard.text(), "one\ntwo\n");
}

#[test]
fn each_cursor_pastes_its_own_piece() {
    let mut editor = editor("aa bb", 0);
    select(
        &mut editor,
        SelectionSet::from_selections(
            vec![
                Selection {
                    anchor: 0,
                    head: 2,
                    goal_column: None,
                },
                Selection {
                    anchor: 3,
                    head: 5,
                    goal_column: None,
                },
            ],
            0,
        ),
    );
    editor.copy();
    editor.paste();

    assert_eq!(content(&editor), "aa bb");
    assert_eq!(editor.clipboard.pieces_for(2), ["aa", "bb"]);
}

#[test]
fn one_cursor_pastes_everything_that_was_copied() {
    let mut editor = editor("aa bb", 0);
    select(
        &mut editor,
        SelectionSet::from_selections(
            vec![
                Selection {
                    anchor: 0,
                    head: 2,
                    goal_column: None,
                },
                Selection {
                    anchor: 3,
                    head: 5,
                    goal_column: None,
                },
            ],
            0,
        ),
    );
    editor.copy();

    select(&mut editor, SelectionSet::single(5));
    editor.paste();
    assert_eq!(content(&editor), "aa bbaa\nbb");
}

#[test]
fn pasting_is_one_undo_step() {
    let mut editor = editor("word", 0);
    editor.select_next_occurrence();
    editor.copy();
    select(&mut editor, SelectionSet::single(4));
    editor.paste();
    assert_eq!(content(&editor), "wordword");

    editor.undo();
    assert_eq!(content(&editor), "word");
}

fn type_text(editor: &mut Editor, text: &str) {
    for character in text.chars() {
        editor.insert(&character.to_string());
    }
}

#[test]
fn typing_a_word_undoes_in_one_step() {
    let mut editor = editor("", 0);
    type_text(&mut editor, "hello");

    editor.undo();
    assert_eq!(content(&editor), "");
}

#[test]
fn a_word_boundary_starts_a_new_step() {
    let mut editor = editor("", 0);
    type_text(&mut editor, "foo bar");

    editor.undo();
    assert_eq!(content(&editor), "foo ");
    editor.undo();
    assert_eq!(content(&editor), "foo");
    editor.undo();
    assert_eq!(content(&editor), "");
}

#[test]
fn moving_the_cursor_starts_a_new_step() {
    let mut editor = editor("", 0);
    type_text(&mut editor, "ab");
    editor.move_selections(Motion::Grapheme(Direction::Backward), false);
    editor.insert("c");
    assert_eq!(content(&editor), "acb");

    editor.undo();
    assert_eq!(content(&editor), "ab");
}

#[test]
fn a_line_break_stands_alone() {
    let mut editor = editor("", 0);
    type_text(&mut editor, "ab");
    editor.insert("\n");
    type_text(&mut editor, "cd");

    editor.undo();
    assert_eq!(content(&editor), "ab\n");
    editor.undo();
    assert_eq!(content(&editor), "ab");
}

#[test]
fn pasted_text_stands_alone() {
    let mut editor = editor("", 0);
    type_text(&mut editor, "ab");
    editor.insert("pasted");

    editor.undo();
    assert_eq!(content(&editor), "ab");
}

#[test]
fn backspaces_undo_together() {
    let mut editor = editor("hello", 5);
    for _ in 0..3 {
        editor.delete(Motion::Grapheme(Direction::Backward));
    }
    assert_eq!(content(&editor), "he");

    editor.undo();
    assert_eq!(content(&editor), "hello");
}

#[test]
fn typing_after_deleting_starts_a_new_step() {
    let mut editor = editor("ab", 2);
    editor.delete(Motion::Grapheme(Direction::Backward));
    editor.insert("c");

    editor.undo();
    assert_eq!(content(&editor), "a");
}

#[test]
fn redo_restores_a_whole_group() {
    let mut editor = editor("", 0);
    type_text(&mut editor, "word");
    editor.undo();
    assert_eq!(content(&editor), "");

    editor.redo();
    assert_eq!(content(&editor), "word");
    assert_eq!(editor.view().selections.primary().head, 4);
}

#[test]
fn editing_after_undo_forgets_the_redo() {
    let mut editor = editor("x", 1);
    editor.insert("y");
    editor.undo();
    editor.insert("z");

    assert_eq!(content(&editor), "xz");
    editor.redo();
    assert_eq!(content(&editor), "xz");
}
