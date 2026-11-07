//! The syntax tree of a buffer, kept in step with its text, and the
//! tokens the highlight query reads off it. The tree is parsed once when
//! the language is set and then edited along with the text: tree-sitter is
//! told what was replaced by how much, in the same terms as the journal,
//! and reparses only what that touched.

use std::fmt;
use std::ops::Range;

use tree_sitter::{InputEdit, Node, Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

use super::language::Language;
use crate::text::Text;
use crate::text::transaction::Change;

/// The grammar for a language, from its crate, and the highlight query
/// for it, from `queries`: the crates' own queries do not agree on which
/// of two patterns naming a node wins. Linked in for now; loaded from
/// files, they would be found the same way.
fn grammar(language: &Language) -> Option<(tree_sitter::Language, &'static str)> {
    match language.name {
        #[cfg(feature = "rust")]
        "Rust" => Some((
            tree_sitter_rust::LANGUAGE.into(),
            include_str!("../../queries/rust/highlights.scm"),
        )),
        #[cfg(feature = "json")]
        "JSON" => Some((
            tree_sitter_json::LANGUAGE.into(),
            include_str!("../../queries/json/highlights.scm"),
        )),
        #[cfg(feature = "python")]
        "Python" => Some((
            tree_sitter_python::LANGUAGE.into(),
            include_str!("../../queries/python/highlights.scm"),
        )),
        _ => None,
    }
}

/// Whether this build has the grammar for `language`: each is linked in
/// by a feature of its own.
pub fn has_grammar(language: &Language) -> bool {
    grammar(language).is_some()
}

/// A stretch of text and what the query calls it: `keyword`, `string`,
/// `function.method`. The names are the grammar's own; what to make of
/// them is a theme's business.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub range: Range<usize>,
    pub name: &'a str,
}

pub struct Syntax {
    parser: Parser,
    tree: Tree,
    query: Query,
}

impl Syntax {
    /// `None` for a language without a grammar.
    pub fn new(language: &Language, text: &Text) -> Option<Self> {
        let (grammar, highlights) = grammar(language)?;
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
            .expect("a grammar linked in matches the runtime it is linked with");
        // Both are linked in, and tests/highlights loads every query of the
        // build: one that does not fit is a bug of the build.
        let query = Query::new(&grammar, highlights).unwrap_or_else(|error| {
            panic!(
                "the {} query does not fit its grammar: {error}",
                language.name
            )
        });
        let tree = parse(&mut parser, text, None);
        Some(Self {
            parser,
            tree,
            query,
        })
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The tokens touching `range`, in order of their start; those starting
    /// together in the order of the query. A token may lie partly outside
    /// `range`, and one may lie inside another: a `!` inside a macro call.
    /// Several patterns naming the same node yield one token, from the
    /// last of them, which is how tree-sitter's own highlighter reads a
    /// query and so how the queries are written: the general pattern
    /// first, the specific ones after it (tree-sitter-highlight 0.25,
    /// "keep iterating over any later highlighting patterns"). A node and
    /// its only child may cover the same bytes and are still two tokens,
    /// the child's after the parent's.
    pub fn tokens(&self, text: &Text, range: Range<usize>) -> Vec<Token<'_>> {
        let names = self.query.capture_names();
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(range);
        // Predicates such as `#match?` want the text of a node, which is
        // read where it lies.
        let mut captures = cursor.captures(&self.query, self.tree.root_node(), |node: Node| {
            text.chunks_in(node.byte_range())
        });

        let mut tokens: Vec<Token> = Vec::new();
        // The captures of one node come out together, in the order of the
        // patterns, so the last node named is the only one to check against
        // and the last capture of it is the one that stands.
        let mut named: Option<Node> = None;
        while let Some((found, index)) = captures.next() {
            let capture = found.captures()[*index];
            let name = names[capture.index as usize];
            if named == Some(capture.node) {
                tokens.last_mut().expect("the node has a token").name = name;
                continue;
            }
            named = Some(capture.node);
            tokens.push(Token {
                range: capture.node.byte_range(),
                name,
            });
        }
        tokens
    }

    /// Brings the tree up to `text` once an edit has been applied to it,
    /// given the inverse of that edit: where the new text lies and what
    /// was there. Both are in the coordinates of the new text, front to
    /// back, which is also how the tree takes them one after another —
    /// when a change is reached, the ones before it are already in the
    /// tree and the ones after it lie beyond its reach.
    pub fn update(&mut self, text: &Text, inverse: &[Change]) {
        for change in inverse {
            let start = point(text, change.range.start);
            self.tree.edit(&InputEdit {
                start_byte: change.range.start,
                old_end_byte: change.range.start + change.text.len(),
                new_end_byte: change.range.end,
                start_position: start,
                old_end_position: advance(start, &change.text),
                new_end_position: point(text, change.range.end),
            });
        }
        self.tree = parse(&mut self.parser, text, Some(&self.tree));
    }
}

/// Reading the text chunk by chunk, where it lies, rather than copying it.
fn parse(parser: &mut Parser, text: &Text, old: Option<&Tree>) -> Tree {
    parser
        .parse_with_options(&mut |byte, _| text.chunk_at(byte), old, None)
        .expect("parsing is neither cancelled nor timed out")
}

fn point(text: &Text, byte: usize) -> Point {
    let row = text.line_of(byte);
    Point {
        row,
        column: byte - text.line_start(row),
    }
}

/// Where `text` ends when it starts at `start`.
fn advance(start: Point, text: &str) -> Point {
    match text.rsplit_once('\n') {
        Some((before, tail)) => Point {
            row: start.row + before.matches('\n').count() + 1,
            column: tail.len(),
        },
        None => Point {
            row: start.row,
            column: start.column + text.len(),
        },
    }
}

impl fmt::Debug for Syntax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Syntax")
            .field("tree", &self.tree.root_node().to_sexp())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;

    #[test]
    fn a_language_without_a_grammar_has_no_tree() {
        let buffer = Buffer::from_text(Text::from_str("plain\n"));
        assert!(buffer.syntax().is_none());
    }

    #[test]
    fn an_insertion_advances_the_point_by_its_shape() {
        let start = Point { row: 3, column: 5 };
        assert_eq!(advance(start, "ab"), Point { row: 3, column: 7 });
        assert_eq!(advance(start, "ab\n"), Point { row: 4, column: 0 });
        assert_eq!(advance(start, "a\nbc\nd"), Point { row: 5, column: 1 });
    }
}

/// The parts that need a real grammar to read, on Rust's.
#[cfg(all(test, feature = "rust"))]
mod rust_tests {
    use std::path::Path;

    use super::*;
    use crate::buffer::language;
    use crate::buffer::{Buffer, EditKind};
    use crate::text::selection::{Selection, SelectionSet};
    use crate::text::transaction::{CursorAfter, Transaction};

    fn rust_buffer(content: &str) -> Buffer {
        let mut buffer = Buffer::from_text(Text::from_str(content));
        buffer.set_language(language::detect(Path::new("x.rs")));
        buffer
    }

    /// Every node with where it is, so that two trees can be compared in
    /// full: the shape, and the bytes and points the shape is pinned to.
    fn nodes(tree: &Tree) -> Vec<(String, usize, usize, Point, Point)> {
        let mut cursor = tree.walk();
        let mut out = Vec::new();
        loop {
            let node = cursor.node();
            out.push((
                node.kind().to_owned(),
                node.start_byte(),
                node.end_byte(),
                node.start_position(),
                node.end_position(),
            ));
            if cursor.goto_first_child() || cursor.goto_next_sibling() {
                continue;
            }
            loop {
                if !cursor.goto_parent() {
                    return out;
                }
                if cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn assert_fresh(buffer: &Buffer) {
        let fresh = Syntax::new(buffer.language(), buffer.text()).unwrap();
        assert_eq!(
            nodes(buffer.syntax().unwrap().tree()),
            nodes(fresh.tree()),
            "{:?}",
            buffer.text().to_string()
        );
    }

    #[test]
    fn the_tree_follows_an_edit_that_adds_lines() {
        let mut buffer = rust_buffer("fn main() {}\n");
        let at = "fn main() {".len();
        let transaction =
            Transaction::edit(&SelectionSet::single(at), CursorAfter::EndOfChange, |_| {
                (at..at, "\n    let x = 1;\n".to_owned())
            });
        buffer.apply(transaction, EditKind::Other);

        assert_eq!(
            buffer.text().to_string(),
            "fn main() {\n    let x = 1;\n}\n"
        );
        assert!(!buffer.syntax().unwrap().tree().root_node().has_error());
        assert_fresh(&buffer);
    }

    #[test]
    fn the_tree_follows_several_cursors_at_once() {
        let mut buffer = rust_buffer("let a = 1;\nlet b = 2;\n");
        // Both names selected, each replaced by a longer one.
        let mut selections: SelectionSet = Selection {
            anchor: 4,
            head: 5,
            goal_column: None,
        }
        .into();
        selections.push(Selection {
            anchor: 15,
            head: 16,
            goal_column: None,
        });
        let transaction = Transaction::edit(&selections, CursorAfter::EndOfChange, |selection| {
            (selection.range(), "longer".to_owned())
        });
        buffer.apply(transaction, EditKind::Other);

        assert_eq!(
            buffer.text().to_string(),
            "let longer = 1;\nlet longer = 2;\n"
        );
        assert_fresh(&buffer);
    }

    #[test]
    fn undo_edits_the_tree_too() {
        let mut buffer = rust_buffer("fn a() {}\n");
        let transaction =
            Transaction::edit(&SelectionSet::single(9), CursorAfter::EndOfChange, |_| {
                (9..9, "\nfn b() {}".to_owned())
            });
        buffer.apply(transaction, EditKind::Other);
        buffer.undo();

        assert_eq!(buffer.text().to_string(), "fn a() {}\n");
        assert_fresh(&buffer);
    }

    /// The token covering exactly `word`, which must occur once.
    fn token_of<'a>(buffer: &Buffer, tokens: &[Token<'a>], word: &str) -> Option<Token<'a>> {
        let text = buffer.text().to_string();
        let at = text.find(word).expect("the word is in the text");
        assert_eq!(text[at + 1..].find(word), None, "{word:?} occurs twice");
        tokens
            .iter()
            .find(|token| token.range == (at..at + word.len()))
            .cloned()
    }

    #[test]
    fn only_the_tokens_touching_the_range_are_read() {
        let buffer = rust_buffer("fn main() {\n    let s = \"hi\";\n}\n");
        let syntax = buffer.syntax().unwrap();
        let second_line = buffer.text().line_start(1)..buffer.text().line_start(2);
        let tokens = syntax.tokens(buffer.text(), second_line);

        assert_eq!(token_of(&buffer, &tokens, "fn"), None);
        assert!(token_of(&buffer, &tokens, "let").is_some());
        // Sorted by where they start.
        assert!(
            tokens
                .windows(2)
                .all(|pair| pair[0].range.start <= pair[1].range.start)
        );
    }

    #[test]
    fn a_node_named_by_several_patterns_takes_the_last() {
        // `Foo` is a constructor by one pattern and, as the path of a
        // scoped identifier, a type by a later one; both are decided by a
        // `#match?` on its text, which is read from the rope.
        let buffer = rust_buffer("Foo::bar();\n");
        let syntax = buffer.syntax().unwrap();
        let tokens = syntax.tokens(buffer.text(), 0..buffer.text().len());

        let named: Vec<_> = tokens
            .iter()
            .filter(|token| token.range == (0..3))
            .map(|token| token.name)
            .collect();
        assert_eq!(named, ["type"]);
    }
}
