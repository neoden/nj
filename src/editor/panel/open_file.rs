use super::field::Field;

/// The field a path to open is typed into. `Enter` opens the file and
/// closes it.
#[derive(Debug)]
pub struct OpenFile {
    pub path: Field,
}

impl OpenFile {
    pub fn new() -> Self {
        Self {
            path: Field {
                label: "Open:",
                ..Field::default()
            },
        }
    }
}
