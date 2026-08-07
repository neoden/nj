/// What was copied, kept as one piece per cursor so that a multi-cursor copy
/// can be pasted back into the same number of cursors.
#[derive(Debug, Default)]
pub struct Clipboard {
    pieces: Vec<String>,
    /// Copied with nothing selected, meaning whole lines were taken and whole
    /// lines should be put back.
    lines: bool,
}

impl Clipboard {
    pub fn set(&mut self, pieces: Vec<String>, lines: bool) {
        self.pieces = pieces;
        self.lines = lines;
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub fn holds_lines(&self) -> bool {
        self.lines
    }

    pub fn text(&self) -> String {
        match self.lines {
            true => self.pieces.concat(),
            false => self.pieces.join("\n"),
        }
    }

    /// Cursor for cursor when the counts agree; otherwise every cursor gets
    /// the whole thing.
    pub fn pieces_for(&self, cursors: usize) -> Vec<String> {
        match self.pieces.len() == cursors {
            true => self.pieces.clone(),
            false => vec![self.text(); cursors],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clipboard(pieces: &[&str], lines: bool) -> Clipboard {
        let mut clipboard = Clipboard::default();
        clipboard.set(pieces.iter().map(|&piece| piece.to_owned()).collect(), lines);
        clipboard
    }

    #[test]
    fn matching_cursor_counts_get_a_piece_each() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(2), ["one", "two"]);
    }

    #[test]
    fn a_lone_cursor_gets_everything() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(1), ["one\ntwo"]);
    }

    #[test]
    fn mismatched_counts_get_everything_each() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(3), ["one\ntwo", "one\ntwo", "one\ntwo"]);
    }

    #[test]
    fn copied_lines_already_carry_their_breaks() {
        let clipboard = clipboard(&["one\n", "two\n"], true);
        assert_eq!(clipboard.text(), "one\ntwo\n");
    }
}
