//! Runs the scenario on standard input and prints what the screen showed.
//! The instructions are described in `term::scenario`.

use std::io::{self, Read};

use anyhow::Result;

use nj::term::sandbox;

fn main() -> Result<()> {
    let mut script = String::new();
    io::stdin().read_to_string(&mut script)?;
    for snapshot in sandbox::run(&script)? {
        match snapshot.caption.as_str() {
            "" => println!("---"),
            caption => println!("--- {caption}"),
        }
        println!("{}", snapshot.frame);
    }
    Ok(())
}
