use super::field::Field;

/// The search field: one line holding the query. Typing in it does nothing
/// to the text; `Enter` starts the search and closes it.
#[derive(Debug)]
pub struct Find {
    pub query: Field,
}

impl Find {
    /// What it opens holding is selected, so that typing replaces it.
    pub fn new(seed: Option<&str>) -> Self {
        let mut query = Field {
            label: "Find:",
            ..Field::default()
        };
        if let Some(seed) = seed {
            query.insert(seed);
            query.select_all();
        }
        Self { query }
    }
}
