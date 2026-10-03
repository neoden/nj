//! The strip above the status bar: a prompt, a picker or a question.

mod fuzzy;
pub mod picker;
pub mod prompt;
pub mod replacing;

use std::path::PathBuf;

use picker::Picker;
use prompt::{Field, Prompt};
use replacing::Replacing;

/// A question with single-key answers. `Esc` always means "never mind".
/// Each choice carries what it does, so there is no separate record of
/// what the question was about to keep in step with the answers.
#[derive(Debug)]
pub struct Confirm {
    pub question: &'static str,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub key: char,
    pub label: &'static str,
    pub answer: Answer,
}

/// What answering does. Every variant is an action the editor carries out
/// as it is, with no question left to ask which one was meant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Save, then carry on with what the save was for.
    Save(After),
    /// Leave the changes behind.
    Quit,
    /// Leave the changes behind and open this file instead.
    Open(PathBuf),
    /// Write over what another writer put on disk, then carry on.
    Overwrite(After),
    /// Save under this name although a file is there already.
    Replace(PathBuf, After),
    /// Leave with a failure, changes and all.
    Abort,
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
    fn new(question: &'static str, choices: Vec<(char, &'static str, Answer)>) -> Self {
        let choices = choices
            .into_iter()
            .map(|(key, label, answer)| Choice { key, label, answer })
            .collect();
        Self { question, choices }
    }

    /// Unsaved changes stand in the way of quitting.
    pub fn before_quitting() -> Self {
        Self::new(
            "Unsaved changes",
            vec![
                ('s', "Save and quit", Answer::Save(After::Quit)),
                ('q', "Quit anyway", Answer::Quit),
            ],
        )
    }

    /// Unsaved changes stand in the way of another file taking this one's
    /// place.
    pub fn before_opening(path: PathBuf) -> Self {
        Self::new(
            "Unsaved changes",
            vec![
                (
                    's',
                    "Save and open",
                    Answer::Save(After::Open(path.clone())),
                ),
                ('d', "Discard and open", Answer::Open(path)),
            ],
        )
    }

    /// The file changed on disk since it was read. Asked on the way out as
    /// well, when "save and quit" runs into it.
    pub fn overwrite(after: After) -> Self {
        Self::new(
            "File changed on disk",
            vec![('o', "Overwrite", Answer::Overwrite(after))],
        )
    }

    /// Saving as a file that exists.
    pub fn replace(path: PathBuf, after: After) -> Self {
        Self::new(
            "File exists",
            vec![('o', "Overwrite", Answer::Replace(path, after))],
        )
    }

    /// Unsaved changes would go with the abort.
    pub fn abort() -> Self {
        Self::new(
            "Unsaved changes",
            vec![('a', "Abort anyway", Answer::Abort)],
        )
    }

    /// What `key` answers, if it answers anything.
    pub fn answer(&self, key: char) -> Option<Answer> {
        self.choices
            .iter()
            .find(|choice| choice.key == key)
            .map(|choice| choice.answer.clone())
    }

    /// The question and its answers, one per row, with the labels lined up.
    /// Height and drawing read the same list, so they cannot disagree.
    pub fn rows(&self) -> Vec<String> {
        let answers: Vec<(String, &str)> = self
            .choices
            .iter()
            .map(|choice| (choice.key.to_string(), choice.label))
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
    Replacing(Replacing),
}

impl Panel {
    /// Includes the rules above and below, which keep the contents off both
    /// the text and the status bar.
    pub fn height(&self) -> usize {
        2 + match self {
            Panel::Prompt(prompt) => prompt.height(),
            Panel::Picker(picker) => picker.height(),
            Panel::Confirm(confirm) => confirm.rows().len(),
            Panel::Replacing(_) => 1,
        }
    }

    /// The one-line field with the keyboard, for panels that have one. A new
    /// kind adds an arm here, in `height`, and its own drawing — and nothing
    /// else.
    pub fn field_mut(&mut self) -> Option<&mut Field> {
        match self {
            Panel::Prompt(prompt) => Some(prompt.field_mut()),
            Panel::Picker(picker) => Some(&mut picker.query),
            Panel::Confirm(_) | Panel::Replacing(_) => None,
        }
    }

    pub fn as_prompt(&self) -> Option<&Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
            _ => None,
        }
    }
}
