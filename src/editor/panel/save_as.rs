use super::After;
use super::field::Field;

/// The field a name to save under is typed into. `Enter` saves and closes
/// it.
#[derive(Debug)]
pub struct SaveAs {
    pub path: Field,
    /// What the save is for, since a buffer with no name yet may be asked
    /// for one on the way out.
    pub after: After,
}

impl SaveAs {
    /// A file's own name is the likeliest start of the new one.
    pub fn new(after: After, name: Option<&str>) -> Self {
        let mut path = Field {
            label: "Save as:",
            ..Field::default()
        };
        if let Some(name) = name {
            path.insert(name);
        }
        Self { path, after }
    }
}
