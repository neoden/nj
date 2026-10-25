//! Runs the scenario on standard input and prints what the screen showed.
//! The instructions are described in `term::scenario`.

use std::io::{self, Read};

use anyhow::Result;

use ned::term::scenario;

fn main() -> Result<()> {
    let mut script = String::new();
    io::stdin().read_to_string(&mut script)?;
    print!("{}", scenario::run(&script)?.join("\n"));
    Ok(())
}
