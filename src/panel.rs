use crate::picker::Picker;
use crate::prompt::Prompt;

/// The strip above the status bar. Whatever is open here owns the keyboard,
/// so there is never a question of where typing goes.
#[derive(Debug)]
pub enum Panel {
    Prompt(Prompt),
    Picker(Picker),
}

impl Panel {
    /// Includes the rules above and below, which keep the contents off both
    /// the text and the status bar.
    pub fn height(&self) -> usize {
        2 + match self {
            Panel::Prompt(prompt) => prompt.height(),
            Panel::Picker(picker) => picker.height(),
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
