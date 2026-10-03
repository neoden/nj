use super::field::Field;

/// The line number field. `Enter` goes to the line and closes it.
#[derive(Debug)]
pub struct GotoLine {
    pub line: Field,
}

impl GotoLine {
    pub fn new() -> Self {
        Self {
            line: Field {
                label: "Line:",
                ..Field::default()
            },
        }
    }
}
