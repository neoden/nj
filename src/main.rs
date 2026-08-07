mod document;
mod key;
mod terminal;
mod text;

use std::io::{self, Write};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue, style};

use crate::document::Document;
use crate::key::Key;
use crate::terminal::Terminal;

fn main() -> Result<()> {
    terminal::install_panic_hook();

    let document = match std::env::args().nth(1) {
        Some(path) => Document::open(path)?,
        None => Document::default(),
    };

    let term = Terminal::enter()?;
    let result = run(&term, &document);
    drop(term);
    result
}

fn run(term: &Terminal, document: &Document) -> Result<()> {
    let mut out = io::stdout();
    let mut last_event = String::from("press any key");
    let mut last_key = String::new();

    loop {
        let (width, height) = crossterm::terminal::size()?;
        let protocol = if term.keyboard_enhanced {
            "enhanced"
        } else {
            "legacy"
        };

        queue!(
            out,
            Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print(format!(
                "{} | {} lines | {:?} | keyboard: {protocol}",
                document
                    .path
                    .as_ref()
                    .map_or("[scratch]".into(), |p| p.display().to_string()),
                document.text.line_count(),
                document.line_ending,
            )),
        )?;

        let visible = (height as usize).saturating_sub(4);
        for line in 0..visible.min(document.text.line_count()) {
            let text: String = document
                .text
                .line(line)
                .chars()
                .take(width as usize)
                .collect();
            queue!(out, cursor::MoveTo(0, line as u16 + 1), style::Print(text))?;
        }

        queue!(
            out,
            cursor::MoveTo(0, height - 3),
            style::Print(&last_event),
            cursor::MoveTo(0, height - 2),
            style::Print(&last_key),
            cursor::MoveTo(0, height - 1),
            style::Print("ctrl+q to quit"),
        )?;
        out.flush()?;

        match event::read()? {
            Event::Key(event) if is_quit(&event) => return Ok(()),
            Event::Key(event) => {
                last_event = format!("{event:?}");
                last_key = format!(
                    "normalized: {:?}  text: {:?}",
                    Key::from_event(&event),
                    key::text_char(&event)
                );
            }
            other => {
                last_event = format!("{other:?}");
                last_key.clear();
            }
        }
    }
}

fn is_quit(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL)
}
