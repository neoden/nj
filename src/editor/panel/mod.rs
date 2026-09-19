//! The strip above the status bar: a prompt, a picker or a question.

pub mod picker;
pub mod prompt;

use std::path::PathBuf;

use picker::Picker;
use prompt::{Field, Prompt};

/// A question with single-key answers. `Esc` always means "never mind".
#[derive(Debug)]
pub struct Confirm {
    pub question: &'static str,
    pub answers: &'static [(char, &'static str)],
    pub about: Question,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// Unsaved changes stand in the way of leaving the buffer behind.
    Leave(After),
    /// The file changed on disk since it was read. Asked on the way out as
    /// well, when "save and quit" runs into it.
    Overwrite(After),
    /// Saving as a file that exists.
    Replace(PathBuf, After),
}

/// What a save was in aid of, to be carried on with once it has gone
/// through — and not before, since it may raise a question of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum After {
    Stay,
    Quit,
    Open(PathBuf),
}

impl Confirm {
    /// `after` says what the answer leads to; `Stay` has nothing to leave
    /// for and is not asked about.
    pub fn leave(after: After) -> Self {
        let answers: &'static [(char, &'static str)] = match after {
            After::Stay | After::Quit => &[('s', "Save and quit"), ('q', "Quit anyway")],
            After::Open(_) => &[('s', "Save and open"), ('d', "Discard and open")],
        };
        Self {
            question: "Unsaved changes",
            answers,
            about: Question::Leave(after),
        }
    }

    pub fn overwrite(after: After) -> Self {
        Self {
            question: "File changed on disk",
            answers: &[('o', "Overwrite")],
            about: Question::Overwrite(after),
        }
    }

    pub fn replace(path: PathBuf, after: After) -> Self {
        Self {
            question: "File exists",
            answers: &[('o', "Overwrite")],
            about: Question::Replace(path, after),
        }
    }

    /// The question and its answers, one per row, with the labels lined up.
    /// Height and drawing read the same list, so they cannot disagree.
    pub fn rows(&self) -> Vec<String> {
        let answers: Vec<(String, &str)> = self
            .answers
            .iter()
            .map(|&(key, label)| (key.to_string(), label))
            .chain([("esc".to_owned(), "Cancel")])
            .collect();

        let keys = answers
            .iter()
            .map(|(key, _)| key.chars().count())
            .max()
            .unwrap_or(0);

        [format!(" {}", self.question)]
            .into_iter()
            .chain(
                answers
                    .iter()
                    .map(|(key, label)| format!("   {key:keys$}   {label}")),
            )
            .collect()
    }
}

/// The strip above the status bar. Whatever is open here owns the keyboard:
/// the keymap binds text commands outside a panel only, and the commands
/// that mean something in both places go to the panel's field first.
#[derive(Debug)]
pub enum Panel {
    Prompt(Prompt),
    Picker(Picker),
    Confirm(Confirm),
}

impl Panel {
    /// Includes the rules above and below, which keep the contents off both
    /// the text and the status bar.
    pub fn height(&self) -> usize {
        2 + match self {
            Panel::Prompt(prompt) => prompt.height(),
            Panel::Picker(picker) => picker.height(),
            Panel::Confirm(confirm) => confirm.rows().len(),
        }
    }

    /// The one-line field with the keyboard, for panels that have one. A new
    /// kind adds an arm here, in `height`, and its own drawing — and nothing
    /// else.
    pub fn field_mut(&mut self) -> Option<&mut Field> {
        match self {
            Panel::Prompt(prompt) => Some(prompt.field_mut()),
            Panel::Picker(picker) => Some(&mut picker.query),
            Panel::Confirm(_) => None,
        }
    }

    pub fn as_prompt(&self) -> Option<&Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
            _ => None,
        }
    }
}
