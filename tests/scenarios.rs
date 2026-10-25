//! Runs every `tests/scenarios/*.sc` through `term::scenario` and compares
//! what the screen showed with the `.out` beside it (ADR 25).
//!
//!   cargo test --test scenarios                 all of them
//!   cargo test --test scenarios -- find         those with `find` in the name
//!   cargo test --test scenarios -- --update find   write what they show now
//!
//! A missing `.out` is a failure: a new scenario does not pass on nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};

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
    let expected_path = script.with_extension("out");
    let shown = scenario::run(&fs::read_to_string(script)?)?;
    if update {
        fs::write(&expected_path, &shown)?;
        return Ok("written");
    }
    let expected = fs::read_to_string(&expected_path).with_context(|| {
        format!(
            "no {} — run with --update to write it",
            expected_path.display()
        )
    })?;
    if shown == expected {
        return Ok("ok");
    }
    anyhow::bail!("{}", differences(&expected, &shown));
}

/// The lines that differ, by their number: `-` for what was expected and
/// `+` for what was shown.
fn differences(expected: &str, shown: &str) -> String {
    let mut expected = expected.lines();
    let mut shown = shown.lines();
    let mut out = String::new();
    for number in 1.. {
        let (want, got) = (expected.next(), shown.next());
        if want.is_none() && got.is_none() {
            break;
        }
        if want == got {
            continue;
        }
        if let Some(want) = want {
            out.push_str(&format!("{number:>3} - {want}\n"));
        }
        if let Some(got) = got {
            out.push_str(&format!("{number:>3} + {got}\n"));
        }
    }
    out
}
