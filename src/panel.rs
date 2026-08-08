use crate::prompt::Prompt;

/// The strip above the status bar. Whatever is open here owns the keyboard,
/// so there is never a question of where typing goes.
#[derive(Debug)]
pub enum Panel {
    Prompt(Prompt),
}

impl Panel {
    /// Includes the rule above the panel and the blank row below it, which
    /// keeps the contents off the status bar.
    pub fn height(&self) -> usize {
        2 + match self {
            Panel::Prompt(prompt) => prompt.height(),
        }
    }

    pub fn as_prompt(&self) -> Option<&Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
        }
    }

    pub fn as_prompt_mut(&mut self) -> Option<&mut Prompt> {
        match self {
            Panel::Prompt(prompt) => Some(prompt),
        }
    }
}
