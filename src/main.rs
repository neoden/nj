use anyhow::Result;

use ned::app;
use ned::document::Document;
use ned::editor::Editor;
use ned::terminal::{self, Terminal};

fn main() -> Result<()> {
    terminal::install_panic_hook();

    let document = match std::env::args().nth(1) {
        Some(path) => Document::open(path)?,
        None => Document::default(),
    };

    let term = Terminal::enter()?;
    let mut editor = Editor::new(document).with_system_clipboard();
    if !term.keyboard_enhanced {
        editor.message = Some("Legacy keyboard: some combinations are unavailable".to_owned());
    }

    // The terminal is restored before the error is printed, or the message
    // would land on the alternate screen and vanish with it.
    let result = app::run(&mut editor);
    drop(term);
    result
}
