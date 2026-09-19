use crate::picker::Picker;
use crate::prompt::{Field, Prompt};

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
    /// The file changed on disk since it was read. Asked on the way out as
    /// well, when "save and quit" runs into it.
    Overwrite { then_quit: bool },
}

impl Confirm {
    pub fn quit() -> Self {
        Self {
            question: "Unsaved changes",
            answers: &[('s', "Save and quit"), ('q', "Quit anyway")],
            about: Question::Quit,
        }
    }

    pub fn overwrite(then_quit: bool) -> Self {
        Self {
            question: "File changed on disk",
            answers: &[('o', "Overwrite")],
            about: Question::Overwrite { then_quit },
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
