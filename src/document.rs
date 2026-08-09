use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::indent;
use crate::language::{self, Indent, Language};
use crate::movement::{Class, Direction};
use crate::selection::SelectionSet;
use crate::text::{Text, TextBuilder};
use crate::transaction::Transaction;

const BOM: &str = "\u{feff}";

/// Every step holds the text it removed, so the history is bounded.
const UNDO_LIMIT: usize = 1000;

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
    undo_stack: Vec<Group>,
    redo_stack: Vec<Group>,
    last_edit: Option<(EditKind, SelectionSet)>,
    next_revision: Revision,
    /// The revision the file on disk holds, or `None` when that is the document
    /// as it was opened.
    saved: Option<Revision>,
}

/// Names an undo group, so that saving can remember which edit reached the
/// disk and undoing back to it can be recognized.
type Revision = u64;

/// One undo step: everything that will be undone together, newest first.
#[derive(Debug)]
struct Group {
    revision: Revision,
    transactions: Vec<Transaction>,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit: None,
            next_revision: 0,
            saved: None,
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
        let file = match fs::File::open(path) {
            Ok(file) => file,
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

        let mut document = Self {
            path: Some(path.to_path_buf()),
            language: language::detect(path),
            ..Self::read(file).with_context(|| format!("cannot read {}", path.display()))?
        };
        document.indent = indent::detect(&document.text, document.language.indent);
        Ok(document)
    }

    fn read(reader: impl Read) -> io::Result<Self> {
        let (text, line_ending, has_bom) = read_text(reader)?;
        Ok(Self {
            line_ending,
            has_bom,
            ..Self::from_text(text)
        })
    }

    #[cfg(test)]
    fn parse(content: &str) -> Self {
        Self::read(content.as_bytes()).expect("reading from memory cannot fail")
    }

    /// True while the buffer holds something other than what is on disk.
    /// Undoing back to the edit that was saved clears it again, which is why
    /// this is a comparison rather than a flag.
    pub fn is_modified(&self) -> bool {
        self.undo_stack.last().map(|group| group.revision) != self.saved
    }

    /// Returns where the cursors end up.
    pub fn apply(&mut self, transaction: Transaction, kind: EditKind) -> SelectionSet {
        let selections = transaction.selections_after().clone();
        let joins = self.joins_previous(&transaction, kind);
        let inverse = transaction.apply(&mut self.text);

        match self.undo_stack.last_mut().filter(|_| joins) {
            // Groups are kept in the order they will be applied, and undoing
            // runs newest first.
            Some(group) => group.transactions.insert(0, inverse),
            None => {
                self.next_revision += 1;
                self.undo_stack.push(Group {
                    revision: self.next_revision,
                    transactions: vec![inverse],
                });
                if self.undo_stack.len() > UNDO_LIMIT {
                    self.undo_stack.remove(0);
                }
            }
        }

        self.redo_stack.clear();
        self.last_edit = Some((kind, selections.clone()));
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
        Some(selections)
    }

    pub fn redo(&mut self) -> Option<SelectionSet> {
        let group = self.redo_stack.pop()?;
        let (inverse, selections) = self.apply_group(group);
        self.undo_stack.push(inverse);
        self.last_edit = None;
        Some(selections)
    }

    /// The group that comes back carries the same revision, so that undoing and
    /// redoing across a save is recognized as arriving where it started.
    fn apply_group(&mut self, group: Group) -> (Group, SelectionSet) {
        let mut selections = SelectionSet::single(0);
        let mut transactions = Vec::with_capacity(group.transactions.len());

        for transaction in group.transactions {
            selections = transaction.selections_after().clone();
            transactions.push(transaction.apply(&mut self.text));
        }

        transactions.reverse();
        (
            Group {
                revision: group.revision,
                transactions,
            },
            selections,
        )
    }

    pub fn save(&mut self) -> Result<()> {
        let path = self.path.as_ref().context("document has no path")?;
        write_atomically(path, &self.encode())
            .with_context(|| format!("cannot write {}", path.display()))?;
        self.saved = self.undo_stack.last().map(|group| group.revision);
        // Typing after a save starts a group of its own, or it would go on
        // extending the revision that just reached the disk.
        self.last_edit = None;
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

/// Builds the buffer as the file arrives, normalizing on the way, so the text
/// never exists twice over. Returns what the file has to be written back with.
///
/// Every CRLF goes, not just those of the ending that was detected: a file
/// whose first break happens to be bare would otherwise keep the carriage
/// returns of all the others. A lone CR stays, being data.
fn read_text(mut reader: impl Read) -> io::Result<(Text, LineEnding, bool)> {
    const BUFFER: usize = 64 * 1024;
    let invalid = || io::Error::new(ErrorKind::InvalidData, "not valid UTF-8");

    let mut builder = TextBuilder::default();
    let mut buffer = vec![0u8; BUFFER];
    let mut filled = 0;
    let mut line_ending = None;
    let mut has_bom = false;
    let mut first = true;

    loop {
        let read = reader.read(&mut buffer[filled..])?;
        filled += read;

        let mut valid = match std::str::from_utf8(&buffer[..filled]) {
            Ok(_) => filled,
            Err(error) => error.valid_up_to(),
        };
        if read > 0 {
            // A trailing CR may be half of a CRLF whose LF has not arrived.
            if valid > 0 && buffer[valid - 1] == b'\r' {
                valid -= 1;
            }
            if first && valid < BOM.len() {
                continue;
            }
        }

        if valid > 0 {
            let mut chunk = std::str::from_utf8(&buffer[..valid]).map_err(|_| invalid())?;
            if first {
                if let Some(rest) = chunk.strip_prefix(BOM) {
                    has_bom = true;
                    chunk = rest;
                }
                first = false;
            }
            if line_ending.is_none() && chunk.contains('\n') {
                line_ending = Some(LineEnding::detect(chunk));
            }
            match chunk.contains('\r') {
                true => builder.append(&chunk.replace("\r\n", "\n")),
                false => builder.append(chunk),
            }
        }

        if valid < filled {
            buffer.copy_within(valid..filled, 0);
        }
        filled -= valid;

        if read == 0 {
            return match filled {
                0 => Ok((builder.finish(), line_ending.unwrap_or_default(), has_bom)),
                _ => Err(invalid()),
            };
        }
    }
}

/// Stages the contents beside the original and renames over it, so that an
/// interrupted save leaves the old file rather than a truncated one. The
/// staging file is a sibling because a rename is only atomic within one
/// filesystem.
fn write_atomically(path: &Path, contents: &str) -> Result<()> {
    let name = path.file_name().context("path names no file")?.to_string_lossy();
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let staging = directory.join(format!(".{name}.ned-{}", std::process::id()));

    let staged = stage(&staging, path, contents);
    if staged.is_err() {
        let _ = fs::remove_file(&staging);
        return staged;
    }
    fs::rename(&staging, path).map_err(|error| {
        let _ = fs::remove_file(&staging);
        error.into()
    })
}

fn stage(staging: &Path, original: &Path, contents: &str) -> Result<()> {
    let mut file = fs::File::create(staging)?;
    file.write_all(contents.as_bytes())?;
    // Renaming over the original is only safe once the new contents have
    // reached the disk, or a crash could leave an empty file in its place.
    file.sync_all()?;

    // A file that already existed keeps the mode it had; a new one keeps
    // whatever it was created with.
    if let Ok(metadata) = fs::metadata(original) {
        let _ = fs::set_permissions(staging, metadata.permissions());
    }
    Ok(())
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
    fn a_mixed_file_still_loses_every_carriage_return() {
        // The first break is bare, so the ending reads as LF — the CRLF that
        // comes later has to go all the same.
        let document = Document::parse("a\nb\r\nc");
        assert_eq!(document.line_ending, LineEnding::Lf);
        assert_eq!(document.text.to_string(), "a\nb\nc");
    }

    #[test]
    fn a_lone_carriage_return_is_left_alone() {
        assert_eq!(Document::parse("a\rb\n").text.to_string(), "a\rb\n");
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

    /// Hands out one byte per call, so every boundary the reader has to cope
    /// with — a split codepoint, a CRLF, a BOM — lands between two reads.
    struct Trickle<'a>(&'a [u8]);

    impl Read for Trickle<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.0.is_empty() || buffer.is_empty() {
                return Ok(0);
            }
            buffer[0] = self.0[0];
            self.0 = &self.0[1..];
            Ok(1)
        }
    }

    #[test]
    fn a_dribbling_reader_produces_the_same_document() {
        for content in [
            "a\r\nb\r\n",
            "\u{feff}a\r\nb",
            "a\nb\r\nc",
            "a\rb\n",
            "привет\r\nмир",
            "\u{feff}",
            "ab",
            "",
        ] {
            let whole = Document::parse(content);
            let dribbled = Document::read(Trickle(content.as_bytes())).unwrap();

            assert_eq!(dribbled.text.to_string(), whole.text.to_string(), "{content:?}");
            assert_eq!(dribbled.line_ending, whole.line_ending, "{content:?}");
            assert_eq!(dribbled.has_bom, whole.has_bom, "{content:?}");
        }
    }

    #[test]
    fn invalid_utf8_is_refused() {
        assert!(Document::read(&[0xff, 0xfe][..]).is_err());
        assert!(Document::read(Trickle(&[b'a', 0xc3])).is_err());
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ned-{}-{name}", std::process::id()))
    }

    #[test]
    fn saving_writes_the_contents_back() {
        let path = scratch("save");
        fs::write(&path, "old\n").unwrap();

        let mut document = Document::open(&path).unwrap();
        document.text.insert(3, " and new");
        document.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "old and new\n");
        assert!(!document.is_modified());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_did_not_exist_is_created() {
        let path = scratch("new");
        let _ = fs::remove_file(&path);

        let mut document = Document::open(&path).unwrap();
        document.text.insert(0, "fresh\n");
        document.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");

        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn saving_keeps_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;

        let path = scratch("mode");
        fs::write(&path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

        let mut document = Document::open(&path).unwrap();
        document.text.insert(0, "# edited\n");
        document.save().unwrap();

        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        let _ = fs::remove_file(&path);
    }
}
