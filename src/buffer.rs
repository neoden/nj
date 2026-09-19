use std::collections::VecDeque;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

use crate::anchor::{self, Anchor, Splice, Version};
use crate::indent;
use crate::language::{self, Indent, Language};
use crate::movement::{Class, Direction};
use crate::selection::SelectionSet;
use crate::text::{Text, TextBuilder};
use crate::transaction::Transaction;

const BOM: &str = "\u{feff}";

/// Every step holds the text it removed, so the history is bounded.
const UNDO_LIMIT: usize = 1000;

/// Steps the journal keeps. An anchor older than that cannot be resolved,
/// which is said rather than answered wrongly (ADR 18). A step is a few
/// ranges, so this is cheap to keep long.
const JOURNAL_LIMIT: usize = 4096;

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

/// How the bytes were framed when read, and so how they are written back.
/// A buffer created empty gets the defaults: LF, no BOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Framing {
    pub line_ending: LineEnding,
    pub has_bom: bool,
}

/// What the disk held when the file was last read or written, so that a save
/// can tell whether someone else wrote to it in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    modified: SystemTime,
    len: u64,
}

impl Stamp {
    pub fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok()?,
            len: metadata.len(),
        })
    }
}

/// What stands behind the buffer outside the process: where the text came
/// from and where a save goes. The two are coupled, which is why this is one
/// enum rather than a source and a sink.
#[derive(Debug)]
pub enum Backing {
    /// Created empty. A save needs a path first.
    Scratch,
    /// Read from a file and written back to it.
    File {
        path: PathBuf,
        /// `None` while the file does not exist on disk yet, or no longer
        /// does.
        on_disk: Option<Stamp>,
    },
    /// Read from standard input; what was last saved goes to standard output
    /// when the editor exits, so it can stand in a pipeline. Starts as the
    /// input, so quitting without saving passes the text through.
    Stdio { committed: Text },
}

/// One open text: what is being edited, plus everything needed to write it
/// back where it came from.
///
/// The text is private, and the one call that changes it writes the journal
/// (ADR 18): a position taken at some version can then be brought forward to
/// the current one, however long ago and by whomever it was taken.
#[derive(Debug)]
pub struct Buffer {
    text: Text,
    pub framing: Framing,
    pub backing: Backing,
    pub language: &'static Language,
    pub indent: Indent,
    undo_stack: Vec<Group>,
    redo_stack: Vec<Group>,
    last_edit: Option<(EditKind, SelectionSet)>,
    next_revision: Revision,
    /// The revision the file on disk holds, or `None` when that is the document
    /// as it was opened.
    saved: Option<Revision>,
    /// Goes up by one per applied transaction, undo and redo included: they
    /// are edits like any other, not a rewind.
    version: Version,
    /// What each transaction did, oldest first; the front entry took the text
    /// from `version - journal.len()` to the version after it. Geometry only,
    /// so it is not the undo stack and cannot be.
    journal: VecDeque<Vec<Splice>>,
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

impl Default for Buffer {
    fn default() -> Self {
        Self {
            text: Text::default(),
            framing: Framing::default(),
            backing: Backing::Scratch,
            language: &language::PLAIN,
            indent: language::PLAIN.indent,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit: None,
            next_revision: 0,
            saved: None,
            version: 0,
            journal: VecDeque::new(),
        }
    }
}

impl Buffer {
    pub fn from_text(text: Text) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }

    pub fn text(&self) -> &Text {
        &self.text
    }

    pub fn version(&self) -> Version {
        self.version
    }

    /// A position in the current text, from a position taken at an earlier
    /// version. `None` when that version has left the journal.
    pub fn resolve(&self, anchor: Anchor) -> Option<usize> {
        let behind = self.version.checked_sub(anchor.version)?;
        let steps = usize::try_from(behind).ok()?;
        if steps > self.journal.len() {
            return None;
        }
        let since = self.journal.len() - steps;
        Some(self.journal.range(since..).fold(anchor.offset, |offset, step| {
            anchor::displace(offset, step.iter().cloned(), anchor.bias)
        }))
    }

    /// The one place the text changes: whatever is applied is journaled, and
    /// the version moves on.
    fn splice(&mut self, transaction: Transaction) -> Transaction {
        let splices = transaction.splices();
        let inverse = transaction.apply(&mut self.text);
        self.journal.push_back(splices);
        if self.journal.len() > JOURNAL_LIMIT {
            self.journal.pop_front();
        }
        self.version += 1;
        inverse
    }

    /// A missing file opens as an empty buffer, so `ned newfile` works.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let context = || format!("cannot read {}", path.display());

        let mut buffer = match fs::File::open(path) {
            Ok(file) => Self::read(file).with_context(context)?,
            Err(err) if err.kind() == ErrorKind::NotFound => Self::default(),
            Err(err) => return Err(err).with_context(context),
        };
        buffer.backing = Backing::File {
            path: path.to_path_buf(),
            on_disk: Stamp::of(path),
        };
        buffer.set_language(language::detect(path));
        Ok(buffer)
    }

    /// The text of a pipeline, to be handed on when the editor exits.
    pub fn from_stdin() -> Result<Self> {
        let mut buffer = Self::read(io::stdin().lock()).context("cannot read standard input")?;
        buffer.backing = Backing::Stdio {
            committed: buffer.text.clone(),
        };
        Ok(buffer)
    }

    fn read(reader: impl Read) -> io::Result<Self> {
        let (text, framing) = read_text(reader)?;
        Ok(Self {
            framing,
            ..Self::from_text(text)
        })
    }

    #[cfg(test)]
    fn parse(content: &str) -> Self {
        Self::read(content.as_bytes()).expect("reading from memory cannot fail")
    }

    /// The one place a language change is applied: indentation is
    /// re-detected, everything else is read through the language.
    pub fn set_language(&mut self, language: &'static Language) {
        self.language = language;
        self.indent = indent::detect(&self.text, language.indent);
    }

    pub fn tab_width(&self) -> usize {
        self.language.tab_width
    }

    /// What the status bar calls this buffer.
    pub fn name(&self) -> String {
        match &self.backing {
            Backing::Scratch => "[scratch]".to_owned(),
            Backing::File { path, .. } => path.display().to_string(),
            Backing::Stdio { .. } => "[stdin]".to_owned(),
        }
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
        let inverse = self.splice(transaction);

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
            transactions.push(self.splice(transaction));
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

    /// Whether the disk holds something other than what was last read or
    /// written: a file changed by another writer, or one that has appeared
    /// where there was none. A file that has gone missing is no conflict —
    /// the buffer is then the only copy, and saving simply puts it back.
    pub fn changed_on_disk(&self) -> bool {
        let Backing::File { path, on_disk } = &self.backing else {
            return false;
        };
        match (Stamp::of(path), on_disk) {
            (Some(now), Some(seen)) => now != *seen,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }

    /// Writes the text where it came from, whatever the disk holds by now —
    /// the check for that is the caller's, since only it can ask. For a
    /// pipeline this is not a write but a promise: the text is only handed
    /// on when the editor exits.
    pub fn save(&mut self) -> Result<()> {
        match &mut self.backing {
            Backing::Scratch => return Err(anyhow::anyhow!("buffer has no path")),
            Backing::File { path, on_disk } => {
                write_atomically(path, &self.framing.encode(&self.text))
                    .with_context(|| format!("cannot write {}", path.display()))?;
                *on_disk = Stamp::of(path);
            }
            Backing::Stdio { committed } => *committed = self.text.clone(),
        }

        self.saved = self.undo_stack.last().map(|group| group.revision);
        // Typing after a save starts a group of its own, or it would go on
        // extending the revision that just reached the disk.
        self.last_edit = None;
        Ok(())
    }

    /// What a pipeline downstream receives once the editor has exited: the
    /// last saved text, or the input itself when nothing was saved.
    pub fn finish(&self, out: &mut impl Write) -> io::Result<()> {
        if let Backing::Stdio { committed } = &self.backing {
            out.write_all(self.framing.encode(committed).as_bytes())?;
            out.flush()?;
        }
        Ok(())
    }
}

impl Framing {
    fn encode(&self, text: &Text) -> String {
        let body = text.to_string();
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
fn read_text(mut reader: impl Read) -> io::Result<(Text, Framing)> {
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
                0 => Ok((
                    builder.finish(),
                    Framing {
                        line_ending: line_ending.unwrap_or_default(),
                        has_bom,
                    },
                )),
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
    use crate::anchor::Bias;
    use crate::transaction::CursorAfter;

    #[test]
    fn detects_line_endings() {
        assert_eq!(LineEnding::detect("a\nb"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("no breaks"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb\nc"), LineEnding::Crlf);
    }

    #[test]
    fn buffer_holds_no_carriage_returns() {
        assert_eq!(Buffer::parse("a\r\nb\r\n").text.to_string(), "a\nb\n");
    }

    #[test]
    fn a_mixed_file_still_loses_every_carriage_return() {
        // The first break is bare, so the ending reads as LF — the CRLF that
        // comes later has to go all the same.
        let buffer = Buffer::parse("a\nb\r\nc");
        assert_eq!(buffer.framing.line_ending, LineEnding::Lf);
        assert_eq!(buffer.text.to_string(), "a\nb\nc");
    }

    #[test]
    fn a_lone_carriage_return_is_left_alone() {
        assert_eq!(Buffer::parse("a\rb\n").text.to_string(), "a\rb\n");
    }

    #[test]
    fn bom_is_stripped_from_the_buffer() {
        let buffer = Buffer::parse("\u{feff}text");
        assert!(buffer.framing.has_bom);
        assert_eq!(buffer.text.to_string(), "text");
    }

    #[test]
    fn encoding_restores_the_original_bytes() {
        for content in ["a\nb\n", "a\r\nb\r\n", "\u{feff}a\r\nb", "", "no breaks"] {
            let buffer = Buffer::parse(content);
            assert_eq!(buffer.framing.encode(&buffer.text), content);
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
            let whole = Buffer::parse(content);
            let dribbled = Buffer::read(Trickle(content.as_bytes())).unwrap();

            assert_eq!(dribbled.text.to_string(), whole.text.to_string(), "{content:?}");
            assert_eq!(dribbled.framing, whole.framing, "{content:?}");
        }
    }

    #[test]
    fn invalid_utf8_is_refused() {
        assert!(Buffer::read(&[0xff, 0xfe][..]).is_err());
        assert!(Buffer::read(Trickle(&[b'a', 0xc3])).is_err());
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ned-{}-{name}", std::process::id()))
    }

    /// An edit the way the editor makes them: as a transaction, so that the
    /// history and the journal see it.
    fn insert(buffer: &mut Buffer, at: usize, text: &str) {
        let transaction = Transaction::edit(
            &SelectionSet::single(at),
            CursorAfter::EndOfChange,
            |_| (at..at, text.to_owned()),
        );
        buffer.apply(transaction, EditKind::Other);
    }

    #[test]
    fn saving_writes_the_contents_back() {
        let path = scratch("save");
        fs::write(&path, "old\n").unwrap();

        let mut buffer = Buffer::open(&path).unwrap();
        insert(&mut buffer, 3, " and new");
        buffer.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "old and new\n");
        assert!(!buffer.is_modified());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_did_not_exist_is_created() {
        let path = scratch("new");
        let _ = fs::remove_file(&path);

        let mut buffer = Buffer::open(&path).unwrap();
        assert!(matches!(buffer.backing, Backing::File { on_disk: None, .. }));
        insert(&mut buffer, 0, "fresh\n");
        buffer.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");
        assert!(matches!(buffer.backing, Backing::File { on_disk: Some(_), .. }));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn another_writer_is_noticed_and_a_save_catches_up() {
        let path = scratch("stamp");
        fs::write(&path, "old\n").unwrap();

        let mut buffer = Buffer::open(&path).unwrap();
        assert!(!buffer.changed_on_disk());

        // A different length, so the verdict does not hang on the clock.
        fs::write(&path, "rewritten elsewhere\n").unwrap();
        assert!(buffer.changed_on_disk());

        buffer.save().unwrap();
        assert!(!buffer.changed_on_disk());

        // Gone is not changed: nothing is there to be lost.
        fs::remove_file(&path).unwrap();
        assert!(!buffer.changed_on_disk());
    }

    #[test]
    fn a_file_appearing_under_a_new_buffer_counts_as_changed() {
        let path = scratch("appeared");
        let _ = fs::remove_file(&path);

        let buffer = Buffer::open(&path).unwrap();
        assert!(!buffer.changed_on_disk());

        fs::write(&path, "someone else\n").unwrap();
        assert!(buffer.changed_on_disk());

        let _ = fs::remove_file(&path);
    }

    fn anchor(buffer: &Buffer, offset: usize, bias: Bias) -> Anchor {
        Anchor {
            version: buffer.version(),
            offset,
            bias,
        }
    }

    #[test]
    fn every_transaction_moves_the_version_on_including_undo() {
        let mut buffer = Buffer::parse("abc");
        assert_eq!(buffer.version(), 0);

        insert(&mut buffer, 1, "X");
        assert_eq!(buffer.version(), 1);
        buffer.undo();
        assert_eq!(buffer.version(), 2);
        buffer.redo();
        assert_eq!(buffer.version(), 3);
    }

    #[test]
    fn an_anchor_is_carried_through_the_edits_since_its_version() {
        let mut buffer = Buffer::parse("abcdef");
        let e = anchor(&buffer, 4, Bias::After);
        let start = anchor(&buffer, 0, Bias::Before);

        insert(&mut buffer, 0, "__");
        insert(&mut buffer, 3, "X");
        assert_eq!(buffer.text().to_string(), "__aXbcdef");
        assert_eq!(buffer.resolve(e), Some(7));
        assert_eq!(buffer.resolve(start), Some(0));

        // A current anchor resolves to itself.
        assert_eq!(buffer.resolve(anchor(&buffer, 5, Bias::After)), Some(5));
    }

    #[test]
    fn an_anchor_survives_an_undo_because_undo_is_an_edit_too() {
        let mut buffer = Buffer::parse("abcdef");
        insert(&mut buffer, 0, "___");
        let d = anchor(&buffer, 6, Bias::After);

        buffer.undo();
        assert_eq!(buffer.text().to_string(), "abcdef");
        assert_eq!(buffer.resolve(d), Some(3));
    }

    #[test]
    fn an_anchor_older_than_the_journal_cannot_be_resolved() {
        let mut buffer = Buffer::parse("");
        let old = anchor(&buffer, 0, Bias::After);
        for _ in 0..JOURNAL_LIMIT {
            insert(&mut buffer, 0, "x");
        }
        assert_eq!(buffer.resolve(old), Some(JOURNAL_LIMIT));

        insert(&mut buffer, 0, "x");
        assert_eq!(buffer.resolve(old), None);
        // The one after it is still in reach.
        let next = Anchor {
            version: 1,
            ..old
        };
        assert_eq!(buffer.resolve(next), Some(JOURNAL_LIMIT));
    }

    #[test]
    fn a_scratch_buffer_has_nowhere_to_be_saved() {
        let mut buffer = Buffer::default();
        assert_eq!(buffer.name(), "[scratch]");
        assert!(buffer.save().is_err());
    }

    #[test]
    fn a_pipeline_hands_on_what_was_last_saved() {
        let mut buffer = Buffer::parse("in\r\n");
        buffer.backing = Backing::Stdio {
            committed: buffer.text.clone(),
        };

        // Nothing saved: the input passes through, framing included.
        let mut out = Vec::new();
        buffer.finish(&mut out).unwrap();
        assert_eq!(out, b"in\r\n");

        insert(&mut buffer, 0, "edited ");
        let mut out = Vec::new();
        buffer.finish(&mut out).unwrap();
        assert_eq!(out, b"in\r\n");

        buffer.save().unwrap();
        let mut out = Vec::new();
        buffer.finish(&mut out).unwrap();
        assert_eq!(out, b"edited in\r\n");
    }

    #[cfg(unix)]
    #[test]
    fn saving_keeps_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;

        let path = scratch("mode");
        fs::write(&path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

        let mut buffer = Buffer::open(&path).unwrap();
        insert(&mut buffer, 0, "# edited\n");
        buffer.save().unwrap();

        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        let _ = fs::remove_file(&path);
    }
}
