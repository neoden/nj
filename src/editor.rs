use crate::document::Document;
use crate::movement::{self, Motion};
use crate::transaction::Transaction;
use crate::view::View;

pub struct Editor {
    pub document: Document,
    pub view: View,
}

impl Editor {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            view: View::default(),
        }
    }

    pub fn move_selections(&mut self, motion: Motion, extend: bool) {
        self.view
            .move_selections(&self.document.text, motion, extend);
    }

    pub fn insert(&mut self, inserted: &str) {
        let transaction = Transaction::edit(&self.view.selections, |selection| {
            (selection.range(), inserted.to_owned())
        });
        self.apply(transaction);
    }

    /// A cursor with nothing selected eats the neighbouring text instead.
    pub fn delete(&mut self, motion: Motion) {
        let text = &self.document.text;
        let transaction = Transaction::edit(&self.view.selections, |selection| {
            let range = match selection.is_empty() {
                false => selection.range(),
                true => {
                    let target = match motion {
                        Motion::Word(direction) => movement::word(text, selection.head, direction),
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
        });

        if !transaction.is_empty() {
            self.apply(transaction);
        }
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

    fn apply(&mut self, transaction: Transaction) {
        self.view.selections = self.document.apply(transaction);
        self.view.scroll_to_cursor(&self.document.text);
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
