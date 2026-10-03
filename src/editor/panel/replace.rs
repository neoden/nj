use super::field::Field;

/// The replace fields: what to find and what to put in its place. `Tab`
/// goes from one to the other; `Enter` closes them and starts asking at
/// each hit.
#[derive(Debug)]
pub struct Replace {
    pub query: Field,
    pub replacement: Field,
    /// Which of the two has the keyboard.
    pub on_replacement: bool,
}

impl Replace {
    /// What the query opens holding is selected, so that typing replaces it.
    pub fn new(seed: Option<&str>) -> Self {
        let mut query = Field {
            label: "Find:",
            ..Field::default()
        };
        if let Some(seed) = seed {
            query.insert(seed);
            query.select_all();
        }
        let replacement = Field {
            label: "Replace:",
            ..Field::default()
        };
        Self {
            query,
            replacement,
            on_replacement: false,
        }
    }

    /// The field with the keyboard.
    pub fn field_mut(&mut self) -> &mut Field {
        match self.on_replacement {
            true => &mut self.replacement,
            false => &mut self.query,
        }
    }

    pub fn switch_field(&mut self) {
        self.on_replacement = !self.on_replacement;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_fields_take_turns() {
        let mut replace = Replace::new(None);
        replace.field_mut().insert("old");

        replace.switch_field();
        replace.field_mut().insert("new");
        assert_eq!(replace.query.contents(), "old");
        assert_eq!(replace.replacement.contents(), "new");

        replace.switch_field();
        assert!(!replace.on_replacement);
    }
}
