use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }

    /// Mixed files are saved with whichever ending came first.
    fn detect(text: &str) -> Self {
        match text.find('\n') {
            Some(idx) if text[..idx].ends_with('\r') => LineEnding::Crlf,
            _ => LineEnding::Lf,
        }
    }
}

/// One open file: its text plus everything needed to write it back unchanged.
#[derive(Debug, Default)]
pub struct Document {
    pub text: Text,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    pub has_bom: bool,
    pub modified: bool,
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
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
                return Ok(Self {
                    path: Some(path.to_path_buf()),
                    ..Self::default()
                })
            }
            Err(err) => return Err(err).with_context(|| format!("cannot read {}", path.display())),
        };

        let content = String::from_utf8(bytes)
            .with_context(|| format!("{} is not valid UTF-8", path.display()))?;

        Ok(Self {
            path: Some(path.to_path_buf()),
            ..Self::parse(&content)
        })
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
            text,
            line_ending,
            has_bom,
            ..Self::default()
        }
    }

    /// Returns where the cursors end up.
    pub fn apply(&mut self, transaction: Transaction) -> SelectionSet {
        let selections = transaction.selections_after().clone();
        let inverse = transaction.apply(&mut self.text);
        self.undo_stack.push(inverse);
        self.redo_stack.clear();
        self.modified = true;
        selections
    }

    pub fn undo(&mut self) -> Option<SelectionSet> {
        let transaction = self.undo_stack.pop()?;
        let selections = transaction.selections_after().clone();
        let inverse = transaction.apply(&mut self.text);
        self.redo_stack.push(inverse);
        self.modified = true;
        Some(selections)
    }

    pub fn redo(&mut self) -> Option<SelectionSet> {
        let transaction = self.redo_stack.pop()?;
        let selections = transaction.selections_after().clone();
        let inverse = transaction.apply(&mut self.text);
        self.undo_stack.push(inverse);
        self.modified = true;
        Some(selections)
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
