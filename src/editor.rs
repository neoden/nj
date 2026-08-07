use std::ops::Range;

use crate::clipboard::{Clipboard, SystemClipboard};
use crate::document::{Document, EditKind};
use crate::movement::{self, Class, Direction, Motion};
use crate::selection::{Selection, SelectionSet};
use crate::text::Text;
use crate::transaction::{CursorAfter, Transaction};
use crate::view::View;

pub struct Editor {
    pub document: Document,
    pub view: View,
    pub clipboard: Clipboard,
    system: Option<SystemClipboard>,
}

impl Editor {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            view: View::default(),
            clipboard: Clipboard::default(),
            system: None,
        }
    }

    /// Left out by default so that tests never touch the real clipboard.
    pub fn with_system_clipboard(mut self) -> Self {
        self.system = Some(SystemClipboard::new());
        self
    }

    pub fn move_selections(&mut self, motion: Motion, extend: bool) {
        self.view
            .move_selections(&self.document.text, motion, extend);
    }

    pub fn insert(&mut self, inserted: &str) {
        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| (selection.range(), inserted.to_owned()),
        );
        self.apply(transaction, insert_kind(inserted));
    }

    /// With nothing selected, whole lines are taken — and put back as whole
    /// lines, which is what every editor with this binding does.
    pub fn copy(&mut self) {
        let text = &self.document.text;
        let lines = self.view.selections.iter().all(Selection::is_empty);
        let pieces = self
            .view
            .selections
            .iter()
            .map(|selection| {
                let mut piece = text.slice(copied_range(text, selection, lines)).to_string();
                if lines && !piece.ends_with('\n') {
                    piece.push('\n');
                }
                piece
            })
            .collect();

        self.clipboard.set(pieces, lines);

        let text = self.clipboard.text();
        if let Some(system) = &mut self.system {
            system.write(&text);
        }
    }

    pub fn cut(&mut self) {
        self.copy();

        let text = &self.document.text;
        let lines = self.clipboard.holds_lines();
        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| (copied_range(text, selection, lines), String::new()),
        );

        if !transaction.is_empty() {
            self.apply(transaction, EditKind::Other);
        }
    }

    pub fn paste(&mut self) {
        self.adopt_foreign_clipboard();
        if self.clipboard.is_empty() {
            return;
        }

        let lines = self.clipboard.holds_lines();
        let mut pieces = self
            .clipboard
            .pieces_for(self.view.selections.len())
            .into_iter();

        let text = &self.document.text;
        let cursor = match lines {
            true => CursorAfter::Displaced,
            false => CursorAfter::EndOfChange,
        };
        let transaction = Transaction::edit(&self.view.selections, cursor, |selection| {
            let range = match lines {
                true => {
                    let start = text.line_start(text.line_of(selection.head));
                    start..start
                }
                false => selection.range(),
            };
            (range, pieces.next().unwrap_or_default())
        });

        self.apply(transaction, EditKind::Other);
    }

    /// Someone else copied since we did, so our pieces are stale: what the
    /// desktop holds is one piece of text, and it is not made of lines.
    fn adopt_foreign_clipboard(&mut self) {
        let Some(text) = self.system.as_mut().and_then(SystemClipboard::read) else {
            return;
        };
        if text != self.clipboard.text() {
            self.clipboard.set(vec![text], false);
        }
    }

    /// A cursor with nothing selected eats the neighbouring text instead.
    pub fn delete(&mut self, motion: Motion) {
        let text = &self.document.text;
        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| {
                let range = match selection.is_empty() {
                    false => selection.range(),
                    true => {
                        let target = match motion {
                            Motion::Word(direction) => {
                                movement::word(text, selection.head, direction)
                            }
                            Motion::LineEdge(direction) => {
                                movement::line_edge(text, selection.head, direction)
                            }
                            Motion::Grapheme(direction) => {
                                movement::grapheme(text, selection.head, direction)
                            }
                            _ => selection.head,
                        };
                        selection.head.min(target)..selection.head.max(target)
                    }
                };
                (range, String::new())
            },
        );

        let kind = match motion {
            Motion::Grapheme(direction) => EditKind::Delete(direction),
            _ => EditKind::Other,
        };
        if !transaction.is_empty() {
            self.apply(transaction, kind);
        }
    }

    pub fn remove_secondary_cursors(&mut self) {
        self.view.selections.collapse_to_primary();
        self.view.scroll_to_cursor(&self.document.text);
    }

    pub fn cancel_selection(&mut self) {
        self.view.selections.collapse_to_cursors();
    }

    /// Takes the lines a selection touches, break included. Pressing again
    /// swallows the next line, because the selection now ends on its start.
    pub fn select_line(&mut self) {
        let text = &self.document.text;
        self.view.selections = self.view.selections.mapped(|selection| {
            let last = text.line_of(selection.max());
            Selection {
                anchor: text.line_start(text.line_of(selection.min())),
                head: match last + 1 < text.line_count() {
                    true => text.line_start(last + 1),
                    false => text.len(),
                },
                goal_column: None,
            }
        });
        self.view.scroll_to_cursor(&self.document.text);
    }

    pub fn add_cursor(&mut self, direction: Direction) {
        self.view.add_cursor(&self.document.text, direction);
    }

    /// First press selects the word under each cursor; every press after that
    /// adds the next occurrence of it, wrapping around the document.
    pub fn select_next_occurrence(&mut self) {
        let Some(search) = self.take_search() else {
            return;
        };

        let text = &self.document.text;
        let from = self.view.selections.primary().max();
        let Some(at) = search
            .find(text, from)
            .or_else(|| search.find(text, 0))
        else {
            return;
        };

        let found = search.selection_at(at);
        if self
            .view
            .selections
            .iter()
            .any(|selection| selection.range() == found.range())
        {
            return;
        }

        self.view.selections.push(found);
        self.view.scroll_to_cursor(&self.document.text);
    }

    pub fn select_all_occurrences(&mut self) {
        let Some(search) = self.take_search() else {
            return;
        };

        let text = &self.document.text;
        let mut selections = Vec::new();
        let mut at = 0;
        while let Some(found) = search.find(text, at) {
            at = found + search.needle.len();
            selections.push(search.selection_at(found));
        }

        self.view.selections = SelectionSet::from_selections(selections, 0);
        self.view.scroll_to_cursor(&self.document.text);
    }

    /// Bare cursors first expand to the word they touch; only once something
    /// is selected is there anything to search for.
    fn take_search(&mut self) -> Option<Search> {
        let text = &self.document.text;
        if self.view.selections.iter().any(Selection::is_empty) {
            self.view.selections = self.view.selections.mapped(|selection| {
                if !selection.is_empty() {
                    return selection;
                }
                let word = movement::word_around(text, selection.head);
                Selection {
                    anchor: word.start,
                    head: word.end,
                    goal_column: None,
                }
            });
            return None;
        }

        let range = self.view.selections.primary().range();
        Some(Search {
            whole_word: movement::word_around(text, range.start) == range,
            needle: text.slice(range).to_string(),
        })
    }

    pub fn undo(&mut self) {
        if let Some(selections) = self.document.undo() {
            self.view.selections = selections;
            self.view.scroll_to_cursor(&self.document.text);
        }
    }

    pub fn redo(&mut self) {
        if let Some(selections) = self.document.redo() {
            self.view.selections = selections;
            self.view.scroll_to_cursor(&self.document.text);
        }
    }

    fn apply(&mut self, transaction: Transaction, kind: EditKind) {
        self.view.selections = self.document.apply(transaction, kind);
        self.view.scroll_to_cursor(&self.document.text);
    }
}

/// What a copy or a cut takes for one cursor.
fn copied_range(text: &Text, selection: &Selection, lines: bool) -> Range<usize> {
    if !lines {
        return selection.range();
    }
    let line = text.line_of(selection.head);
    let start = text.line_start(line);
    start..(start + text.line_str(line).len() + 1).min(text.len())
}

struct Search {
    needle: String,
    /// A selection that covers exactly one word searches for that word, not
    /// for the letters in it: renaming `i` should not match every `if`.
    whole_word: bool,
}

impl Search {
    fn find(&self, text: &Text, from: usize) -> Option<usize> {
        let mut at = from;
        loop {
            let found = text.find(&self.needle, at)?;
            if !self.whole_word || self.is_whole_word(text, found) {
                return Some(found);
            }
            at = movement::grapheme(text, found, Direction::Forward);
        }
    }

    fn is_whole_word(&self, text: &Text, at: usize) -> bool {
        let before = movement::grapheme(text, at, Direction::Backward);
        let word_before = before != at && text.char_at(before).map(Class::of) == Some(Class::Word);
        let word_after = text.char_at(at + self.needle.len()).map(Class::of) == Some(Class::Word);
        !word_before && !word_after
    }

    fn selection_at(&self, at: usize) -> Selection {
        Selection {
            anchor: at,
            head: at + self.needle.len(),
            goal_column: None,
        }
    }
}

/// Only single characters join a group; a paste or a line break stands alone.
fn insert_kind(inserted: &str) -> EditKind {
    let mut characters = inserted.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) if character != '\n' => EditKind::Insert(Class::of(character)),
        _ => EditKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Direction;
    use crate::selection::{Selection, SelectionSet};
    use crate::text::Text;

    fn editor(content: &str, offset: usize) -> Editor {
        let mut editor = Editor::new(Document::from_text(Text::from_str(content)));
        editor.view.fit(80, 25, editor.document.text.line_count());
        editor.view.selections = SelectionSet::single(offset);
        editor
    }

    fn content(editor: &Editor) -> String {
        editor.document.text.to_string()
    }

    #[test]
    fn typing_lands_after_the_cursor() {
        let mut editor = editor("ac", 1);
        editor.insert("b");

        assert_eq!(content(&editor), "abc");
        assert_eq!(editor.view.selections.primary().head, 2);
    }

    #[test]
    fn typing_replaces_a_selection() {
        let mut editor = editor("hello world", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 5,
            goal_column: None,
        }
        .into();
        editor.insert("bye");

        assert_eq!(content(&editor), "bye world");
        assert!(editor.view.selections.primary().is_empty());
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
        assert!(!editor.document.modified);
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
        editor.view.selections.push(Selection::cursor(2));
        editor.view.selections.push(Selection::cursor(4));
        editor.insert("!");

        assert_eq!(content(&editor), "!a !a !a");
        assert_eq!(editor.view.selections.len(), 3);
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
        editor.view.selections.push(Selection::cursor(3));
        editor.insert("!");
        assert_eq!(content(&editor), "a! a!");

        editor.undo();
        let heads: Vec<_> = editor
            .view
            .selections
            .iter()
            .map(|selection| selection.head)
            .collect();
        assert_eq!(heads, [1, 3]);
    }

    fn ranges(editor: &Editor) -> Vec<std::ops::Range<usize>> {
        editor
            .view
            .selections
            .iter()
            .map(Selection::range)
            .collect()
    }

    #[test]
    fn the_first_press_selects_the_word_under_the_cursor() {
        let mut editor = editor("foo bar foo", 1);
        editor.select_next_occurrence();

        assert_eq!(editor.view.selections.len(), 1);
        assert_eq!(editor.view.selections.primary().range(), 0..3);
    }

    #[test]
    fn a_cursor_resting_after_a_word_still_finds_it() {
        let mut editor = editor("foo bar", 3);
        editor.select_next_occurrence();

        assert_eq!(editor.view.selections.primary().range(), 0..3);
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
    fn a_partial_selection_matches_anywhere() {
        let mut editor = editor("foo foobar", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 2,
            goal_column: None,
        }
        .into();
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

        assert_eq!(editor.view.selections.len(), 1);
    }

    #[test]
    fn a_stacked_cursor_keeps_the_goal_column_over_a_short_line() {
        let mut editor = editor("aaaa\nx\ncccc", 3);
        editor.add_cursor(Direction::Forward);
        editor.add_cursor(Direction::Forward);

        assert_eq!(ranges(&editor), [3..3, 6..6, 10..10]);
    }

    #[test]
    fn selecting_a_line_takes_its_break_too() {
        let mut editor = editor("first\nsecond\nthird\n", 8);
        editor.select_line();

        assert_eq!(editor.view.selections.primary().range(), 6..13);
    }

    #[test]
    fn selecting_again_swallows_the_next_line() {
        let mut editor = editor("first\nsecond\nthird\n", 8);
        editor.select_line();
        editor.select_line();

        assert_eq!(editor.view.selections.primary().range(), 6..19);
    }

    #[test]
    fn a_selection_across_lines_expands_to_all_of_them() {
        let mut editor = editor("first\nsecond\nthird\n", 0);
        editor.view.selections = Selection {
            anchor: 3,
            head: 8,
            goal_column: None,
        }
        .into();
        editor.select_line();

        assert_eq!(editor.view.selections.primary().range(), 0..13);
    }

    #[test]
    fn selecting_the_last_line_stops_at_the_end() {
        let mut editor = editor("first\nlast", 7);
        editor.select_line();
        assert_eq!(editor.view.selections.primary().range(), 6..10);

        editor.select_line();
        assert_eq!(editor.view.selections.primary().range(), 6..10);
    }

    #[test]
    fn selected_lines_can_be_cut_at_every_cursor() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        editor.view.selections.push(Selection::cursor(8));
        editor.select_line();
        editor.cut();

        assert_eq!(content(&editor), "two\n");
    }

    #[test]
    fn copying_and_pasting_a_selection() {
        let mut editor = editor("hello world", 0);
        editor.select_next_occurrence();
        editor.copy();

        editor.view.selections = SelectionSet::single(11);
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
        assert_eq!(editor.view.selections.primary().head, 9);
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
        assert_eq!(editor.view.selections.primary().head, 15);
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
    fn each_cursor_pastes_its_own_piece() {
        let mut editor = editor("aa bb", 0);
        editor.view.selections = SelectionSet::from_selections(
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
        );
        editor.copy();
        editor.paste();

        assert_eq!(content(&editor), "aa bb");
        assert_eq!(editor.clipboard.pieces_for(2), ["aa", "bb"]);
    }

    #[test]
    fn one_cursor_pastes_everything_that_was_copied() {
        let mut editor = editor("aa bb", 0);
        editor.view.selections = SelectionSet::from_selections(
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
        );
        editor.copy();

        editor.view.selections = SelectionSet::single(5);
        editor.paste();
        assert_eq!(content(&editor), "aa bbaa\nbb");
    }

    #[test]
    fn pasting_is_one_undo_step() {
        let mut editor = editor("word", 0);
        editor.select_next_occurrence();
        editor.copy();
        editor.view.selections = SelectionSet::single(4);
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
        assert_eq!(editor.view.selections.primary().head, 4);
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
}
