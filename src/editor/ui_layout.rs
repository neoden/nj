//! How the terminal is divided between the text, a panel and the status bar.
//! A view is handed its area from here rather than working one out itself,
//! which is what lets a second view exist.

/// A rectangle of the terminal, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Area {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

const STATUS_ROWS: usize = 1;
/// Rows of text the panel may never take.
const MIN_TEXT_ROWS: usize = 1;
/// A panel is its two rules and a row of its own; less than that is no panel.
const MIN_PANEL_ROWS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiLayout {
    /// Where the text goes, gutter included.
    pub text: Area,
    /// Rows the panel got, which is not always what it asked for.
    pub panel_height: usize,
}

impl UiLayout {
    /// A panel is cut down to what is left rather than pushing the status bar
    /// off the screen, and a screen too short to draw one in gets none at
    /// all.
    pub fn divide(term_width: usize, term_height: usize, panel: usize) -> Self {
        let spare = term_height.saturating_sub(STATUS_ROWS + MIN_TEXT_ROWS);
        let panel_height = match panel.min(spare) {
            height if height >= MIN_PANEL_ROWS => height,
            _ => 0,
        };

        Self {
            text: Area {
                x: 0,
                y: 0,
                width: term_width,
                height: term_height.saturating_sub(panel_height + STATUS_ROWS),
            },
            panel_height,
        }
    }

    /// The row right under the text, where a panel's top rule goes.
    pub fn panel_top(&self) -> usize {
        self.text.y + self.text.height
    }

    /// The status bar is nailed to the last row so that it never moves when a
    /// panel opens or changes height.
    pub fn status_row(&self) -> usize {
        self.panel_top() + self.panel_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_takes_the_whole_width() {
        let layout = UiLayout::divide(400, 24, 0);
        assert_eq!(layout.text.x, 0);
        assert_eq!(layout.text.width, 400);
    }

    #[test]
    fn a_panel_never_pushes_the_status_bar_off_the_screen() {
        let layout = UiLayout::divide(80, 24, 13);
        assert_eq!(layout.panel_height, 13);
        assert_eq!(layout.text.height, 10);
        assert_eq!(layout.status_row(), 23);

        // Asking for more than there is leaves the text a row and the status
        // bar its own.
        let layout = UiLayout::divide(80, 12, 13);
        assert_eq!(layout.panel_height, 10);
        assert_eq!(layout.text.height, 1);

        // Too short to draw a panel in at all.
        let layout = UiLayout::divide(80, 4, 13);
        assert_eq!(layout.panel_height, 0);
        assert_eq!(layout.text.height, 3);
    }
}
