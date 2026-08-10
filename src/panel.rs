use crossterm::event::KeyEvent;

use crate::key::{Key, KeyCode, Mods};
use crate::movement::Direction;
use crate::picker::Picker;
use crate::prompt::Prompt;

/// What a panel wants done after it has dealt with a keystroke itself.
///
/// Each variant answers to exactly one method of the editor, so that deciding
/// what a key means stays inside the panel and no caller has to ask what kind
/// of panel it is holding (ADR 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Dealt with, and nothing outside the panel is concerned.
    Nothing,
    Close,
    /// The query was edited, so whatever it drives has to run again.
    Changed,
    /// A picker's choice was taken.
    Pick,
    /// A prompt was submitted; backwards for `Shift+Enter`.
    Submit { backwards: bool },
    /// A single-key answer to a question.
    Answer(char),
    GoToMatch(Direction),
    SelectAllMatches,
}

/// A question with single-key answers. `Esc` always means "never mind".
#[derive(Debug)]
pub struct Confirm {
    pub question: &'static str,
    pub answers: &'static [(char, &'static str)],
    pub about: Question,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    Quit,
}

impl Confirm {
    pub fn quit() -> Self {
        Self {
            question: "Unsaved changes",
            answers: &[('s', "Save and quit"), ('q', "Quit anyway")],
            about: Question::Quit,
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

    fn handle_key(&self, key: Key) -> Outcome {
        match key.code {
            KeyCode::Esc => Outcome::Close,
            // Combinations are not answers: Ctrl+Q must not discard the changes
            // it is asking about just because it carries a `q`.
            KeyCode::Char(answer) if !key.mods.intersects(Mods::CTRL | Mods::ALT) => {
                Outcome::Answer(answer)
            }
            _ => Outcome::Nothing,
        }
    }
}

/// The strip above the status bar. Whatever is open here owns the keyboard,
/// so there is never a question of where typing goes.
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

    /// The only place that asks what kind of panel this is. A new kind adds an
    /// arm here, an arm in `height`, and its own drawing — and nothing else.
    pub fn handle_key(&mut self, key: Key, event: &KeyEvent) -> Outcome {
        match self {
            Panel::Prompt(prompt) => prompt.handle_key(key, event),
            Panel::Picker(picker) => picker.handle_key(key, event),
            Panel::Confirm(confirm) => confirm.handle_key(key),
        }
    }

    /// True while a panel still has candidates to read, which is what keeps the
    /// main loop from going back to sleep.
    pub fn is_loading(&self) -> bool {
        match self {
            Panel::Picker(picker) => picker.is_loading(),
            _ => false,
        }
    }

    pub fn load_more(&mut self) {
        if let Panel::Picker(picker) = self {
            picker.pull();
        }
    }

    pub fn as_prompt(&self) -> Option<&Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
            _ => None,
        }
    }
}
