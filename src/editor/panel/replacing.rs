/// The line that asks at each hit of a replacement. It stays open from one
/// answer to the next, and holds what has to last that long.
#[derive(Debug)]
pub struct Replacing {
    pub replacement: String,
    /// Hits not answered for yet, the selected one among them. The walk is
    /// over when it reaches zero, which is what makes it one round.
    pub left: usize,
    /// How many hits were replaced, for the status bar to say at the end.
    pub replaced: usize,
}

impl Replacing {
    /// The question and its keys, on the one row the form takes.
    pub fn line(&self) -> String {
        format!(
            " Replace with \"{}\"?   y Replace   n Skip   a Replace all   esc Stop",
            self.replacement
        )
    }
}
