//! How a cell is drawn, as the editor sees it. Two independent channels:
//! what the text is, which decides its colour, and what state lies under
//! it. Kept apart so that a keyword stays a keyword inside a selection and
//! on the current line, without a variant for every combination. The
//! terminal layer that paints these does not care what they mean.

/// What the text in a cell is: a part of the editor's own furniture, or a
/// kind of token in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ink {
    #[default]
    Text,
    Gutter,
    Rule,
    Status,
    Keyword,
    String,
    Comment,
    Type,
    Function,
    Constant,
}

impl Ink {
    /// The ink for a token as a highlight query names it. The names are a
    /// convention shared by the grammars' own queries, dotted from general
    /// to specific — `function.method`, `comment.documentation` — and the
    /// general part decides. A name not known here is plain text, so a
    /// query is never wrong, only less colourful.
    pub fn named(name: &str) -> Self {
        let general = name.split('.').next().unwrap_or(name);
        match general {
            "keyword" | "attribute" => Ink::Keyword,
            "string" | "escape" => Ink::String,
            "comment" => Ink::Comment,
            "type" | "constructor" => Ink::Type,
            "function" => Ink::Function,
            "constant" | "number" | "boolean" => Ink::Constant,
            _ => Ink::Text,
        }
    }
}

/// What lies under the text: a state of the editor, not of the text. A cell
/// under several of them takes one, the topmost; the order is the view's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    None,
    CurrentLine,
    Matched,
    Selected,
    Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    pub ink: Ink,
    pub mark: Mark,
}

impl CellStyle {
    /// Text of `ink` with nothing under it.
    pub const fn of(ink: Ink) -> Self {
        Self {
            ink,
            mark: Mark::None,
        }
    }

    /// Ordinary text with `mark` under it.
    pub const fn marked(mark: Mark) -> Self {
        Self {
            ink: Ink::Text,
            mark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_known_by_the_general_part_of_its_name() {
        assert_eq!(Ink::named("keyword"), Ink::Keyword);
        assert_eq!(Ink::named("function.method"), Ink::Function);
        assert_eq!(Ink::named("comment.documentation"), Ink::Comment);
        assert_eq!(Ink::named("punctuation.bracket"), Ink::Text);
        assert_eq!(Ink::named("variable"), Ink::Text);
    }
}
