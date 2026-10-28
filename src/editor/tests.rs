//! The editor's own tests: what each command does, driven through the
//! methods that implement it. What a key means, and what the screen shows,
//! is `tests/scenarios`.

use super::*;
use crate::text::Text;
use crate::text::movement::Direction;
use crate::text::selection::{Selection, SelectionSet};
use find::MAX_CURSORS;

fn editor(content: &str, offset: usize) -> Editor {
    let mut editor = Editor::new(Buffer::from_text(Text::from_str(content)));
    editor.resize(80, 25);
    select(&mut editor, SelectionSet::single(offset));
    editor
}

/// Starting a test from a known place is a command like any other.
fn select(editor: &mut Editor, selections: impl Into<SelectionSet>) {
    editor.execute(Command::Select(Select::Set(selections.into())));
}

fn select_range(editor: &mut Editor, anchor: usize, head: usize) {
    select(
        editor,
        Selection {
            anchor,
            head,
            goal_column: None,
        },
    );
}

#[test]
fn selections_from_outside_are_snapped_into_the_text() {
    let mut editor = editor("aф", 0);
    select_range(&mut editor, 2, 99);
    assert_eq!(ranges(&editor), [1..3]);
}

fn ranges(editor: &Editor) -> Vec<std::ops::Range<usize>> {
    editor
        .view()
        .selections()
        .iter()
        .map(Selection::range)
        .collect()
}

/// Goes through `execute` so that the focus routing is what puts the
/// text into the open form's field.
fn type_into_field(editor: &mut Editor, typed: &str) {
    for character in typed.chars() {
        editor.execute(Command::Edit(Edit::Insert(character.to_string())));
    }
}

/// The watcher only says the stamp moved; whether that is another writer
/// or our own save is decided on arrival.
#[test]
fn a_report_of_a_changed_file_is_checked_before_it_is_believed() {
    let path = std::env::temp_dir().join(format!("ned-{}-watched", std::process::id()));
    std::fs::write(&path, "mine\n").unwrap();
    let mut editor = Editor::new(Buffer::open(&path).unwrap());
    let id = editor.workspace.focused_buffer();

    // Our own save moves the stamp, and the report that follows it says
    // nothing.
    editor.insert("!", EditKind::Other);
    editor.execute(Command::File(File::Save));
    editor.receive(Report::FileChanged(id));
    assert_eq!(editor.message(), None);

    std::fs::write(&path, "theirs, and longer\n").unwrap();
    editor.receive(Report::FileChanged(id));
    assert_eq!(editor.message(), Some("File changed on disk"));

    let _ = std::fs::remove_file(&path);
}

/// The picker takes candidates from its own worker only: a batch from a
/// picker that has since been closed is not the new one's.
#[test]
fn a_picker_ignores_candidates_meant_for_another() {
    fn picker(editor: &Editor) -> &Picker {
        match &editor.panel {
            Some(Panel::Picker(picker)) => picker,
            _ => panic!("a picker is open"),
        }
    }

    let mut editor = editor("", 0);
    editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
    let stale = picker(&editor).job;

    // Opened again: the first worker is cancelled, but a batch of its
    // may still be in the queue.
    editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
    editor.finish_jobs();
    assert_ne!(picker(&editor).job, stale);
    let total = picker(&editor).total();
    assert!(total > 0);

    editor.receive(Report::Candidates {
        job: stale,
        batch: vec![picker::Candidate::plain("Stray")],
    });
    assert_eq!(picker(&editor).total(), total);
}

/// A prompt opened over a picker — reachable through a command, if not a
/// key — takes its place and lets its worker go, as a picker would.
#[test]
fn a_prompt_replaces_whatever_panel_was_open() {
    let mut editor = editor("", 0);
    editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
    let Some(Panel::Picker(picker)) = &editor.panel else {
        panic!("a picker is open");
    };
    let job = picker.job;

    editor.execute(Command::Panel(PanelCommand::GotoLine));
    assert!(matches!(editor.panel(), Some(Panel::GotoLine(_))));
    assert!(!editor.jobs.is_live(job));
}

fn scratch_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ned-{}-{name}", std::process::id()))
}

/// What the frame cannot show: the file is watched from the moment it is
/// opened, and opening another lets the watcher go.
#[test]
fn opening_another_file_lets_the_watcher_go() {
    let path = scratch_path("open");
    std::fs::write(&path, "one\ntwo\n").unwrap();
    let other = scratch_path("open-other");
    std::fs::write(&other, "other\n").unwrap();

    let mut editor = editor("", 0);
    editor.execute(Command::File(File::Open(path.clone())));
    let id = editor.workspace.focused_buffer();
    let watcher = editor.watchers[&id];
    assert!(editor.jobs.is_live(watcher));

    editor.execute(Command::File(File::Open(other.clone())));
    assert!(!editor.jobs.is_live(watcher));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&other);
}

/// A signal is the one exit that does not ask: there is nobody to ask. It
/// ends as a quit, so what was last saved is what stands.
#[test]
fn a_termination_quits_without_asking_whatever_is_unsaved() {
    let mut dirty = editor("text", 0);
    dirty.insert("!", EditKind::Other);
    dirty.terminate();

    assert_eq!(dirty.exit(), Some(Exit::Quit));
    assert!(dirty.panel.is_none());
}

#[test]
fn a_long_query_is_cut_short_in_the_status() {
    assert_eq!(quoted("foo "), "\"foo \"");
    assert_eq!(quoted(&"a".repeat(24)), format!("\"{}\"", "a".repeat(24)));
    assert_eq!(quoted(&"a".repeat(25)), format!("\"{}…\"", "a".repeat(23)));
    // A wide character goes whole or not at all.
    assert_eq!(
        quoted(&"日".repeat(13)),
        format!("\"{}…\"", "日".repeat(11))
    );
}

/// P10: a cursor on each of a hundred thousand hits is not a selection
/// anyone can edit with, so neither command makes one.
#[test]
fn too_many_hits_are_refused_rather_than_partly_selected() {
    let mut over = editor(&"x ".repeat(MAX_CURSORS + 1), 0);

    over.select_next_occurrence();
    assert_eq!(ranges(&over), [0..1]);
    over.select_all_occurrences();
    assert_eq!(ranges(&over), [0..1]);
    assert!(over.message().unwrap().starts_with("More than"));

    over.open_find();
    type_into_field(&mut over, "x");
    over.execute(Command::Panel(PanelCommand::Search(Direction::Forward)));
    assert_eq!(over.message(), None);
    over.execute(Command::Select(Select::AllOccurrences));
    assert_eq!(over.view().selections().len(), 1);
    assert!(over.message().unwrap().starts_with("More than"));
    // Refused, so the search was not used up.
    assert!(over.has_search());

    // One fewer and the command goes through.
    let mut under = editor(&"x ".repeat(MAX_CURSORS), 0);
    under.select_next_occurrence();
    under.select_all_occurrences();
    assert_eq!(under.view().selections().len(), MAX_CURSORS);
}
