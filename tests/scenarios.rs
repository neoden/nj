//! Runs every `.sc` under `tests/scenarios` through `term::scenario` and
//! checks that the frames written in the file are what the screen shows.
//! Scenarios are grouped in a directory per area, and a scenario
//! is named by its path from there: `search/esc-ends-the-search`.
//!
//!   cargo test --test scenarios                    all of them
//!   cargo test --test scenarios -- search/         one area
//!   cargo test --test scenarios -- esc             names containing `esc`
//!   cargo test --test scenarios -- --update esc    rewrite their frames
//!
//! A file is its instructions, comments and `---` snapshot lines, which are
//! the author's, and under each snapshot the frame it showed, which is
//! generated. Checking a scenario is rendering it afresh and comparing with
//! the file; updating it is writing that rendering back. A snapshot with no
//! frame under it fails, so a new scenario does not pass on nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;

use nj::term::{sandbox, scenario};

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
    for (name, script) in scripts(&filters)? {
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

/// The scenarios with their names, in name order, that match any of
/// `filters` — all of them when there are none.
fn scripts(filters: &[String]) -> Result<Vec<(String, PathBuf)>> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/scenarios"));
    let mut scripts = Vec::new();
    collect(root, &mut scripts)?;
    let mut scripts: Vec<(String, PathBuf)> = scripts
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(root).expect("found under root");
            let name = name.with_extension("").to_string_lossy().into_owned();
            (name, path)
        })
        .filter(|(name, _)| {
            filters.is_empty() || filters.iter().any(|filter| name.contains(filter.as_str()))
        })
        .collect();
    scripts.sort();
    Ok(scripts)
}

/// Every `.sc` under `directory`, at any depth.
fn collect(directory: &Path, into: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, into)?;
        } else if path.extension().is_some_and(|extension| extension == "sc") {
            into.push(path);
        }
    }
    Ok(())
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
/// lines kept as they are, the old frames dropped, and under each snapshot
/// the frame it produced, closed by the blank line that marks its end.
fn rendered(written: &str) -> Result<String> {
    let mut snapshots = sandbox::run(written)?.into_iter();
    let mut out = String::new();
    for line in scenario::lines(written) {
        match line {
            scenario::Line::Text(text) => {
                out.push_str(text);
                out.push('\n');
            }
            scenario::Line::Snapshot(layer, caption) => {
                let snapshot = snapshots.next().expect("a frame per snapshot");
                out.push_str(&scenario::snapshot_line(layer, caption));
                out.push('\n');
                out.push_str(&snapshot.frame);
                out.push('\n');
            }
            scenario::Line::Frame(_) => {}
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
