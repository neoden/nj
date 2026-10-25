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
    /// What either field opens holding is selected, so that typing replaces
    /// it: the query its seed, the replacement what was replaced with last.
    pub fn new(seed: Option<&str>, replacement: Option<&str>) -> Self {
        Self {
            query: seeded("Find:", seed),
            replacement: seeded("Replace:", replacement),
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

fn seeded(label: &'static str, text: Option<&str>) -> Field {
    let mut field = Field {
        label,
        ..Field::default()
    };
    if let Some(text) = text {
        field.insert(text);
        field.select_all();
    }
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_fields_take_turns() {
        let mut replace = Replace::new(None, None);
        replace.field_mut().insert("old");

        replace.switch_field();
        replace.field_mut().insert("new");
        assert_eq!(replace.query.contents(), "old");
        assert_eq!(replace.replacement.contents(), "new");

        replace.switch_field();
        assert!(!replace.on_replacement);
    }
}
