use crate::picker::Picker;
use crate::prompt::Prompt;

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

    pub fn as_picker_mut(&mut self) -> Option<&mut Picker> {
        match self {
            Panel::Picker(picker) => Some(picker),
            _ => None,
        }
    }

    pub fn as_prompt(&self) -> Option<&Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
            _ => None,
        }
    }

    pub fn as_prompt_mut(&mut self) -> Option<&mut Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
            _ => None,
        }
    }
}
