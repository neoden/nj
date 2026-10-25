//! Runs every `tests/scenarios/*.sc` through `term::scenario` and checks
//! that the frames written in the file are what the screen shows (ADR 25).
//!
//!   cargo test --test scenarios                 all of them
//!   cargo test --test scenarios -- find         those with `find` in the name
//!   cargo test --test scenarios -- --update find   rewrite their frames
//!
//! A file is its instructions and comments, which are the author's, and
//! after each `screen` the frame it showed, which is generated. Checking a
//! scenario is rendering it afresh and comparing with the file; updating it
//! is writing that rendering back. A `screen` with no frame under it fails,
//! so a new scenario does not pass on nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;

use ned::term::scenario;

fn main() -> Result<ExitCode> {
    let mut update = false;
    let mut filters = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--update" => update = true,
            flag if flag.starts_with("--") => anyhow::bail!("unknown flag `{flag}`"),
            name => filters.push(name.to_owned()),
        }
    }

    let mut failed = 0;
    for script in scripts(&filters)? {
        let name = script.file_stem().unwrap().to_string_lossy();
        match check(&script, update) {
            Ok(outcome) => println!("{name} ... {outcome}"),
            Err(error) => {
                failed += 1;
                println!("{name} ... FAILED\n{error:#}\n");
            }
        }
    }
    Ok(match failed {
        0 => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    })
}

/// The scenarios, in name order, that match any of `filters` — all of them
/// when there are none.
fn scripts(filters: &[String]) -> Result<Vec<PathBuf>> {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/scenarios");
    let mut scripts: Vec<PathBuf> = fs::read_dir(directory)?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<_>>()?;
    scripts.retain(|path| {
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        path.extension().is_some_and(|extension| extension == "sc")
            && (filters.is_empty() || filters.iter().any(|filter| name.contains(filter.as_str())))
    });
    scripts.sort();
    Ok(scripts)
}

fn check(script: &Path, update: bool) -> Result<&'static str> {
    let written = fs::read_to_string(script)?;
    let fresh = rendered(&written)?;
    if update {
        fs::write(script, &fresh)?;
        return Ok("written");
    }
    if fresh == written {
        return Ok("ok");
    }
    anyhow::bail!("{}", differences(&written, &fresh));
}

/// The scenario with its frames as the screen shows them now: the author's
/// lines kept as they are, the frame lines dropped, and after each `screen`
/// the frame it produced.
fn rendered(written: &str) -> Result<String> {
    let mut frames = scenario::run(written)?.into_iter();
    let mut out = String::new();
    for line in written.lines() {
        if scenario::is_frame_line(line) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
        if line.trim_end() == "screen" {
            out.push_str(&frames.next().expect("a frame per screen"));
        }
    }
    Ok(out)
}

/// The lines that differ, by their number in the file: `-` for what is
/// written and `+` for what the screen shows.
fn differences(written: &str, fresh: &str) -> String {
    let mut written = written.lines();
    let mut fresh = fresh.lines();
    let mut out = String::new();
    for number in 1.. {
        let (have, show) = (written.next(), fresh.next());
        if have.is_none() && show.is_none() {
            break;
        }
        if have == show {
            continue;
        }
        if let Some(have) = have {
            out.push_str(&format!("{number:>3} - {have}\n"));
        }
        if let Some(show) = show {
            out.push_str(&format!("{number:>3} + {show}\n"));
        }
    }
    out
}
