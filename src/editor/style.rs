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
    /// What the language provides by name: `self`, `len`, `None`.
    Builtin,
    /// The name of a loop to break out of: `'outer` in Rust.
    Label,
}

impl Ink {
    /// The ink for a token as a highlight query names it. The names are
    /// the queries' in `queries`, dotted from general to specific —
    /// `function.method`, `comment.documentation` — and the longest start
    /// of the name known here decides, counted in whole parts:
    /// `function.method` is a function, `function.builtin` a builtin. A
    /// name not known here is plain text, so a query is never wrong, only
    /// less colourful.
    pub fn named(name: &str) -> Self {
        let mut name = name;
        loop {
            if let Some(ink) = Self::known(name) {
                return ink;
            }
            match name.rsplit_once('.') {
                Some((general, _)) => name = general,
                None => return Ink::Text,
            }
        }
    }

    fn known(name: &str) -> Option<Self> {
        Some(match name {
            "variable.builtin" | "function.builtin" | "constant.builtin" => Ink::Builtin,
            "label" => Ink::Label,
            // Documentation reads as a comment, though Python writes it as
            // a string.
            "string.documentation" => Ink::Comment,
            "keyword" | "attribute" => Ink::Keyword,
            "string" | "escape" => Ink::String,
            "comment" => Ink::Comment,
            "type" | "constructor" => Ink::Type,
            "function" => Ink::Function,
            "constant" | "number" | "boolean" => Ink::Constant,
            _ => return None,
        })
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
    fn a_token_is_known_by_the_longest_start_of_its_name() {
        assert_eq!(Ink::named("keyword"), Ink::Keyword);
        assert_eq!(Ink::named("function.method"), Ink::Function);
        assert_eq!(Ink::named("comment.documentation"), Ink::Comment);
        assert_eq!(Ink::named("punctuation.bracket"), Ink::Text);
        assert_eq!(Ink::named("variable"), Ink::Text);
        // The whole name is known, so it wins over its general part.
        assert_eq!(Ink::named("function.builtin"), Ink::Builtin);
        assert_eq!(Ink::named("variable.builtin"), Ink::Builtin);
        // Not known whole, so the part before the last dot decides.
        assert_eq!(Ink::named("function.builtin.extra"), Ink::Builtin);
        assert_eq!(Ink::named("type.builtin"), Ink::Type);
        assert_eq!(Ink::named("type.lifetime"), Ink::Type);
        assert_eq!(Ink::named("label"), Ink::Label);
        assert_eq!(Ink::named("string.documentation"), Ink::Comment);
        assert_eq!(Ink::named("string.special.key"), Ink::String);
        // Parts are whole: a name is never cut inside a part.
        assert_eq!(Ink::named("functional"), Ink::Text);
    }
}
