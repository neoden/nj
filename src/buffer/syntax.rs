//! The syntax tree of a buffer, kept in step with its text, and the
//! tokens the highlight query reads off it. The tree is parsed once when
//! the language is set and then edited along with the text: tree-sitter is
//! told what was replaced by how much, in the same terms as the journal,
//! and reparses only what that touched.
//!
//! Some pieces of the text are another grammar's: the text inside a
//! Markdown block. Each is parsed whole, on its own, the first time it is
//! drawn, and kept until the next edit. Where the query combines them —
//! the HTML between a PHP file's tags — they are parsed together, as one
//! document with the outer grammar's text left out of it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

use tree_sitter::{InputEdit, Node, Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

use super::grammar::{self, Grammar};
use super::language::{self, Language};
use crate::text::Text;
use crate::text::transaction::Change;

/// Whether this build has the grammar for `language`: each is linked in
/// by a feature of its own.
pub fn has_grammar(language: &Language) -> bool {
    grammar::is_linked(language.name)
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
    grammar: &'static Grammar,
    parser: Parser,
    tree: Tree,
    /// The pieces another grammar reads, by where they lie, parsed when
    /// first drawn. An edit drops them all: those it did not touch would
    /// still do, but telling them from the rest is following the edit
    /// through every piece, which this is spared. Within one version of
    /// the text a place holds one piece, read by one grammar.
    injected: RefCell<HashMap<(usize, usize), Parsed>>,
    /// The pieces the query combines, one per pattern, from all of the
    /// text: what the first frame after an edit reads, all frames up to
    /// the next edit draw from. `None` until then.
    combined: RefCell<Option<Vec<Parsed>>>,
}

/// A piece as its grammar read it.
struct Parsed {
    grammar: &'static Grammar,
    tree: Tree,
    spans: Vec<tree_sitter::Range>,
}

impl Parsed {
    fn tokens(&self, text: &Text, range: Range<usize>) -> Vec<Token<'static>> {
        let found = highlight(self.grammar, &self.tree, text, range);
        clip(found, &self.spans)
    }
}

/// A piece of the text for another grammar: where it lies, and the spans
/// of it the grammar reads, which leave out what belongs to the outer one,
/// the `> ` starting each line of a quote.
struct Piece {
    grammar: &'static Grammar,
    range: Range<usize>,
    spans: Vec<tree_sitter::Range>,
}

impl Syntax {
    /// `None` for a language without a grammar.
    pub fn new(language: &Language, text: &Text) -> Option<Self> {
        let grammar = grammar::by_name(language.name)?;
        let mut parser = Parser::new();
        parser
            .set_language(&grammar.language)
            .expect("a grammar linked in matches the runtime it is linked with");
        let tree = parse(&mut parser, text, None);
        Some(Self {
            grammar,
            parser,
            tree,
            injected: RefCell::default(),
            combined: RefCell::default(),
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
    /// the child's after the parent's. The tokens of a piece another
    /// grammar reads lie inside the piece, and so come after the outer
    /// grammar's that start where they do: they are the inner ones. They
    /// lie only on the bytes of the piece the grammar read, cut in parts
    /// where they would cover the outer grammar's.
    pub fn tokens(&self, text: &Text, range: Range<usize>) -> Vec<Token<'_>> {
        let mut tokens = highlight(self.grammar, &self.tree, text, range.clone());
        let Some(injections) = &self.grammar.injections else {
            return tokens;
        };
        let mut injected = self.injected.borrow_mut();
        for piece in pieces(injections, &self.tree, text, range.clone(), false) {
            let parsed = injected
                .entry((piece.range.start, piece.range.end))
                .or_insert_with(|| Parsed {
                    grammar: piece.grammar,
                    tree: parse_piece(&piece, text),
                    spans: piece.spans,
                });
            tokens.extend(parsed.tokens(text, range.clone()));
        }
        let mut combined = self.combined.borrow_mut();
        let combined = combined.get_or_insert_with(|| {
            // Reading all of the text is what a combined piece costs, and
            // a query with none would pay it for nothing.
            if !combines(injections) {
                return Vec::new();
            }
            pieces(injections, &self.tree, text, 0..text.len(), true)
                .into_iter()
                .map(|piece| Parsed {
                    grammar: piece.grammar,
                    tree: parse_piece(&piece, text),
                    spans: piece.spans,
                })
                .collect()
        });
        for parsed in combined.iter() {
            tokens.extend(parsed.tokens(text, range.clone()));
        }
        // Stable, so that the outer tokens stay before the inner ones that
        // start where they do, and those of one grammar in its order.
        tokens.sort_by_key(|token| token.range.start);
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
        self.injected.get_mut().clear();
        *self.combined.get_mut() = None;
    }
}

/// The tokens `grammar`'s highlight query names in `tree` that touch
/// `range`, as `Syntax::tokens` describes them.
fn highlight(
    grammar: &'static Grammar,
    tree: &Tree,
    text: &Text,
    range: Range<usize>,
) -> Vec<Token<'static>> {
    let query = &grammar.highlights;
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    // Predicates such as `#match?` want the text of a node, which is read
    // where it lies.
    let mut captures = cursor.captures(query, tree.root_node(), |node: Node| {
        text.chunks_in(node.byte_range())
    });

    let mut tokens: Vec<Token> = Vec::new();
    // The captures of one node come out together, in the order of the
    // patterns, so the last node named is the only one to check against and
    // the last capture of it is the one that stands.
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

/// The parts of `tokens` that lie in `spans`, the bytes the grammar read.
/// A node spans the bytes it was not given as well, when they fall inside
/// it: the string of `href="<?= $url ?>"`, the code of a quote that goes on
/// past a line's `> `. Those bytes are the outer grammar's, and the outer
/// grammar need not name every one of them to cover them.
fn clip(tokens: Vec<Token<'static>>, spans: &[tree_sitter::Range]) -> Vec<Token<'static>> {
    let mut clipped = Vec::with_capacity(tokens.len());
    for token in tokens {
        let first = spans.partition_point(|span| span.end_byte <= token.range.start);
        for span in spans[first..]
            .iter()
            .take_while(|span| span.start_byte < token.range.end)
        {
            clipped.push(Token {
                range: token.range.start.max(span.start_byte)..token.range.end.min(span.end_byte),
                name: token.name,
            });
        }
    }
    clipped
}

/// The pieces touching `range` that the injection query gives to another
/// grammar, read as tree-sitter's own highlighter reads one: the piece is
/// `@injection.content`, its grammar named by the text of
/// `@injection.language` or else by `#set! injection.language`, and its
/// children left out unless `#set! injection.include-children`. A grammar
/// this build does not have leaves its piece to the outer one.
///
/// Either the pieces of the patterns that do not combine, or those of the
/// patterns that do, `#set! injection.combined`: all the matches of such a
/// pattern make one piece, in the grammar its first match names.
fn pieces(
    query: &Query,
    tree: &Tree,
    text: &Text,
    range: Range<usize>,
    combined: bool,
) -> Vec<Piece> {
    let content = query.capture_index_for_name("injection.content");
    let named = query.capture_index_for_name("injection.language");
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    let mut matches = cursor.matches(query, tree.root_node(), |node: Node| {
        text.chunks_in(node.byte_range())
    });

    let mut pieces: Vec<Piece> = Vec::new();
    // The pattern each piece is of, for a combined one to be found again.
    let mut patterns = Vec::new();
    while let Some(found) = matches.next() {
        let mut name = None;
        let mut node = None;
        for capture in found.captures() {
            if Some(capture.index) == named {
                name = Some(text.substring(capture.node.byte_range()));
            } else if Some(capture.index) == content {
                node = Some(capture.node);
            }
        }
        let mut children = false;
        let mut combines = false;
        for property in query.property_settings(found.pattern_index) {
            match &*property.key {
                "injection.language" if name.is_none() => {
                    name = property.value.as_deref().map(str::to_owned);
                }
                "injection.include-children" => children = true,
                "injection.combined" => combines = true,
                _ => {}
            }
        }
        if combines != combined {
            continue;
        }
        let (Some(name), Some(node)) = (name, node) else {
            continue;
        };
        let Some(grammar) = grammar_named(&name) else {
            continue;
        };
        let spans = spans(node, children);
        if spans.is_empty() {
            continue;
        }
        // Matches come in the order they start, so the spans stay in order.
        if combined && let Some(at) = patterns.iter().position(|&p| p == found.pattern_index) {
            let piece = &mut pieces[at];
            piece.range.end = node.end_byte();
            piece.spans.extend(spans);
            continue;
        }
        patterns.push(found.pattern_index);
        pieces.push(Piece {
            grammar,
            range: node.byte_range(),
            spans,
        });
    }
    pieces
}

/// Whether any pattern of the injection query combines its pieces.
fn combines(query: &Query) -> bool {
    (0..query.pattern_count()).any(|pattern| {
        query
            .property_settings(pattern)
            .iter()
            .any(|property| &*property.key == "injection.combined")
    })
}

/// The grammar for a piece by the name the query gives it: a language's,
/// as a writer names it after ```` ``` ````, whatever its case, or by its
/// files' extension, `py`; or one of those no file is written in,
/// `markdown_inline`.
fn grammar_named(name: &str) -> Option<&'static Grammar> {
    let language = language::named(name);
    grammar::by_name(language.map_or(name, |language| language.name))
}

/// What of `node` its grammar reads: all of it, or all but its named
/// children. Named only, as tree-sitter-md's own parser has it, where
/// tree-sitter's highlighter leaves out every child: the block grammar
/// marks each `*` and `` ` `` inside a block's text with a nameless
/// child, and the text without them has no emphasis and no code left.
fn spans(node: Node, children: bool) -> Vec<tree_sitter::Range> {
    if children {
        return vec![node.range()];
    }
    let mut spans = Vec::new();
    let mut from = (node.start_byte(), node.start_position());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.start_byte() > from.0 {
            spans.push(tree_sitter::Range {
                start_byte: from.0,
                start_point: from.1,
                end_byte: child.start_byte(),
                end_point: child.start_position(),
            });
        }
        from = (child.end_byte(), child.end_position());
    }
    if node.end_byte() > from.0 {
        spans.push(tree_sitter::Range {
            start_byte: from.0,
            start_point: from.1,
            end_byte: node.end_byte(),
            end_point: node.end_position(),
        });
    }
    spans
}

/// A piece parsed on its own, in the coordinates of the whole text, so
/// that its tokens need no shifting.
fn parse_piece(piece: &Piece, text: &Text) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&piece.grammar.language)
        .expect("a grammar linked in matches the runtime it is linked with");
    parser
        .set_included_ranges(&piece.spans)
        .expect("the spans of a node are in order and apart");
    parse(&mut parser, text, None)
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

/// What the pieces another grammar reads come to: the tokens inside a
/// Markdown block.
#[cfg(all(test, feature = "markdown"))]
mod markdown_tests {
    use std::path::Path;

    use super::*;
    use crate::buffer::{Buffer, EditKind};
    use crate::text::selection::SelectionSet;
    use crate::text::transaction::{CursorAfter, Transaction};

    /// The names of the tokens covering `byte`, outer first.
    fn names_at(buffer: &Buffer, byte: usize) -> Vec<String> {
        let text = buffer.text();
        let syntax = buffer.syntax().unwrap();
        syntax
            .tokens(text, 0..text.len())
            .iter()
            .filter(|token| token.range.contains(&byte))
            .map(|token| token.name.to_owned())
            .collect()
    }

    #[test]
    fn an_edit_drops_the_pieces_read_before_it() {
        let mut buffer = Buffer::from_text(Text::from_str("a `b` c\n"));
        buffer.set_language(language::detect(Path::new("x.md")));
        assert_eq!(names_at(&buffer, 3), ["markup.raw.inline"]);

        // The closing backtick becomes an `x`: the text is as long as it
        // was, so the piece lies where it lay, and only dropping what was
        // read before the edit keeps the code from staying.
        let transaction =
            Transaction::edit(&SelectionSet::single(4), CursorAfter::EndOfChange, |_| {
                (4..5, "x".to_owned())
            });
        buffer.apply(transaction, EditKind::Other);
        assert_eq!(buffer.text().to_string(), "a `bx c\n");
        assert!(names_at(&buffer, 3).is_empty());
    }

    #[test]
    fn an_inner_token_leaves_out_what_its_grammar_did_not_read() {
        // The code goes on over the second line's `> > `, which the inline
        // grammar was not given: the code is on both sides of it, and not
        // on it.
        let mut buffer = Buffer::from_text(Text::from_str("> > `one\n> > two`\n"));
        buffer.set_language(language::detect(Path::new("x.md")));
        let marker = "> > `one\n> ".len();
        assert_eq!(names_at(&buffer, marker), ["markup.quote.marker"]);
        assert_eq!(names_at(&buffer, "> > `o".len()), ["markup.raw.inline"]);
        assert_eq!(
            names_at(&buffer, "> > `one\n> > t".len()),
            ["markup.raw.inline"]
        );
    }
}
