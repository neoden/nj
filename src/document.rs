use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::indent;
use crate::language::{self, Indent, Language};
use crate::movement::{Class, Direction};
use crate::selection::SelectionSet;
use crate::text::Text;
use crate::transaction::Transaction;

const BOM: &str = "\u{feff}";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}

impl LineEnding {
    /// Mixed files are saved with whichever ending came first.
    fn detect(text: &str) -> Self {
        match text.find('\n') {
            Some(idx) if text[..idx].ends_with('\r') => LineEnding::Crlf,
            _ => LineEnding::Lf,
        }
    }
}

/// One open file: its text plus everything needed to write it back unchanged.
#[derive(Debug)]
pub struct Document {
    pub text: Text,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    pub has_bom: bool,
    pub language: &'static Language,
    pub indent: Indent,
    pub modified: bool,
    undo_stack: Vec<Vec<Transaction>>,
    redo_stack: Vec<Vec<Transaction>>,
    last_edit: Option<(EditKind, SelectionSet)>,
}

/// What an edit was, for deciding whether the next one continues it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Insert(Class),
    Delete(Direction),
    /// Never joins anything: pastes, line breaks, word deletions.
    Other,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: Text::default(),
            path: None,
            line_ending: LineEnding::default(),
            has_bom: false,
            language: &language::PLAIN,
            indent: language::PLAIN.indent,
            modified: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit: None,
        }
    }
}

impl Document {
    pub fn from_text(text: Text) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }

    /// A missing file opens as an empty buffer, so `ned newfile` works.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let language = language::detect(path);
                return Ok(Self {
                    path: Some(path.to_path_buf()),
                    language,
                    indent: language.indent,
                    ..Self::default()
                });
            }
            Err(err) => return Err(err).with_context(|| format!("cannot read {}", path.display())),
        };

        let content = String::from_utf8(bytes)
            .with_context(|| format!("{} is not valid UTF-8", path.display()))?;

        let mut document = Self {
            path: Some(path.to_path_buf()),
            language: language::detect(path),
            ..Self::parse(&content)
        };
        document.indent = indent::detect(&document.text, document.language.indent);
        Ok(document)
    }

    fn parse(content: &str) -> Self {
        let (content, has_bom) = match content.strip_prefix(BOM) {
            Some(rest) => (rest, true),
            None => (content, false),
        };

        let line_ending = LineEnding::detect(content);
        let text = match line_ending {
            LineEnding::Lf => Text::from_str(content),
            LineEnding::Crlf => Text::from_str(&content.replace("\r\n", "\n")),
        };

        Self {
            line_ending,
            has_bom,
            ..Self::from_text(text)
        }
    }

    /// Returns where the cursors end up.
    pub fn apply(&mut self, transaction: Transaction, kind: EditKind) -> SelectionSet {
        let selections = transaction.selections_after().clone();
        let joins = self.joins_previous(&transaction, kind);
        let inverse = transaction.apply(&mut self.text);

        match self.undo_stack.last_mut().filter(|_| joins) {
            // Groups are kept in the order they will be applied, and undoing
            // runs newest first.
            Some(group) => group.insert(0, inverse),
            None => self.undo_stack.push(vec![inverse]),
        }

        self.redo_stack.clear();
        self.last_edit = Some((kind, selections.clone()));
        self.modified = true;
        selections
    }

    /// Typing continues a group only while nothing moved the cursors and the
    /// edit is still of the same nature — the word boundary ends it.
    fn joins_previous(&self, transaction: &Transaction, kind: EditKind) -> bool {
        let Some((last_kind, last_selections)) = &self.last_edit else {
            return false;
        };
        kind != EditKind::Other && *last_kind == kind && last_selections == transaction.selections_before()
    }

    pub fn undo(&mut self) -> Option<SelectionSet> {
        let group = self.undo_stack.pop()?;
        let (inverse, selections) = self.apply_group(group);
        self.redo_stack.push(inverse);
        self.last_edit = None;
        self.modified = true;
        Some(selections)
    }

    pub fn redo(&mut self) -> Option<SelectionSet> {
        let group = self.redo_stack.pop()?;
        let (inverse, selections) = self.apply_group(group);
        self.undo_stack.push(inverse);
        self.last_edit = None;
        self.modified = true;
        Some(selections)
    }

    fn apply_group(&mut self, group: Vec<Transaction>) -> (Vec<Transaction>, SelectionSet) {
        let mut selections = SelectionSet::single(0);
        let mut inverse = Vec::with_capacity(group.len());

        for transaction in group {
            selections = transaction.selections_after().clone();
            inverse.push(transaction.apply(&mut self.text));
        }

        inverse.reverse();
        (inverse, selections)
    }

    pub fn save(&mut self) -> Result<()> {
        let path = self.path.as_ref().context("document has no path")?;
        fs::write(path, self.encode()).with_context(|| format!("cannot write {}", path.display()))?;
        self.modified = false;
        Ok(())
    }

    fn encode(&self) -> String {
        let body = self.text.to_string();
        let body = match self.line_ending {
            LineEnding::Lf => body,
            LineEnding::Crlf => body.replace('\n', "\r\n"),
        };
        match self.has_bom {
            true => format!("{BOM}{body}"),
            false => body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_line_endings() {
        assert_eq!(LineEnding::detect("a\nb"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("no breaks"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb\nc"), LineEnding::Crlf);
    }

    #[test]
    fn buffer_holds_no_carriage_returns() {
        assert_eq!(Document::parse("a\r\nb\r\n").text.to_string(), "a\nb\n");
    }

    #[test]
    fn bom_is_stripped_from_the_buffer() {
        let doc = Document::parse("\u{feff}text");
        assert!(doc.has_bom);
        assert_eq!(doc.text.to_string(), "text");
    }

    #[test]
    fn encoding_restores_the_original_bytes() {
        for content in ["a\nb\n", "a\r\nb\r\n", "\u{feff}a\r\nb", "", "no breaks"] {
            assert_eq!(Document::parse(content).encode(), content);
        }
    }
}
