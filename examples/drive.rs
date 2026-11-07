//! Runs the scenario on standard input and prints what the screen showed.
//! The instructions are described in `term::scenario`.

use std::io::{self, Read};

use anyhow::{Result, bail};

use nj::term::sandbox;

fn main() -> Result<()> {
    let mut script = String::new();
    io::stdin().read_to_string(&mut script)?;
    let snapshots = match sandbox::run(&script)? {
        sandbox::Run::Done(snapshots) => snapshots,
        sandbox::Run::Skipped(name) => bail!("this build has no {name} grammar"),
    };
    for snapshot in snapshots {
        match snapshot.caption.as_str() {
            "" => println!("---"),
            caption => println!("--- {caption}"),
        }
        println!("{}", snapshot.frame);
    }
    Ok(())
}
