//! Runs every `.test` under `queries` and checks that the highlight query
//! names the code in it as the file says. A file is named by its path from
//! there: `rust/highlights`.
//!
//!   cargo test --test highlights               all of them
//!   cargo test --test highlights -- rust/      names containing `rust/`
//!   cargo test --test highlights -- --code rust/ | nj --language Rust
//!                                              their code alone, to see
//!
//! A file is examples of one language, each a few lines of code and what
//! the query should call them. Fenced, because to Markdown a line starting
//! with `>` is a quote:
//!
//! ```text
//! %language Rust
//!
//! # Constants by name
//! > const MAX_SIZE: u8 = 1;
//!         ^^^^^^^^ constant
//!                   ^^ type.builtin
//! ```
//!
//! ```text
//! %language NAME  the language, by its name; once, before the examples
//! # TEXT          a comment; the first line of the one above an example
//!                 names it
//! > CODE          a line of code, `>` alone for an empty one
//! ^^^ NAME        every character above the carets is named NAME by
//!                 the query, `-` for none; the carets line up with the
//!                 code below its two-column prefix
//! (blank line)    the end of an example
//! ```
//!
//! Each example is parsed on its own, so one that does not parse spoils no
//! other. Where tokens nest, a character takes the innermost, as on the
//! screen. A language whose grammar this build left out is skipped, and
//! without a filter every grammar it has must have a file.

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use unicode_width::UnicodeWidthChar;

use nj::buffer::language::{self, LANGUAGES, Language};
use nj::buffer::syntax::{self, Syntax, Token};
use nj::text::TextBuilder;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/queries");

fn main() -> Result<ExitCode> {
    let mut code = false;
    let mut filters = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--code" => code = true,
            flag if flag.starts_with("--") => bail!("unknown flag `{flag}`"),
            name => filters.push(name.to_owned()),
        }
    }

    let mut paths = Vec::new();
    collect(Path::new(ROOT), &mut paths)?;
    paths.sort();
    let chosen = paths
        .into_iter()
        .map(|path| (name_of(&path), path))
        .filter(|(name, _)| {
            filters.is_empty() || filters.iter().any(|filter| name.contains(filter.as_str()))
        });

    if code {
        return print_code(chosen);
    }

    let mut failed = 0;
    let mut covered = Vec::new();
    for (name, path) in chosen {
        let file = read(&path).with_context(|| format!("{name}: cannot read"))?;
        covered.push(file.language);
        if !syntax::has_grammar(file.language) {
            println!("{name} ... skipped (no {} grammar)", file.language.name);
            continue;
        }
        let failures = check(&file);
        if failures.is_empty() {
            println!("{name} ... ok");
        } else {
            failed += 1;
            println!("{name} ... FAILED");
            for failure in failures {
                println!("{failure}");
            }
        }
    }

    if filters.is_empty() {
        for language in LANGUAGES {
            if syntax::has_grammar(language) && !covered.contains(&language) {
                failed += 1;
                println!(
                    "{} has a grammar and no examples under queries",
                    language.name
                );
            }
        }
    }

    Ok(match failed {
        0 => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    })
}

/// The code of every example in `files`, an empty line between two, for
/// a look at it in the editor. Of one language only, or the editor would
/// colour it all by one of them.
fn print_code(files: impl Iterator<Item = (String, PathBuf)>) -> Result<ExitCode> {
    let mut language: Option<&Language> = None;
    let mut examples = Vec::new();
    for (name, path) in files {
        let file = read(&path).with_context(|| format!("{name}: cannot read"))?;
        match language {
            Some(language) if language != file.language => bail!(
                "{name} is {}, the files before it {}: choose files of one language",
                file.language.name,
                language.name
            ),
            _ => language = Some(file.language),
        }
        examples.extend(file.examples);
    }
    if examples.is_empty() {
        bail!("no file matches");
    }

    let code: Vec<String> = examples
        .iter()
        .map(|example| example.lines.join("\n"))
        .collect();
    println!("{}", code.join("\n\n"));
    Ok(ExitCode::SUCCESS)
}

/// Every `.test` under `directory`, at any depth.
fn collect(directory: &Path, into: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, into)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "test")
        {
            into.push(path);
        }
    }
    Ok(())
}

fn name_of(path: &Path) -> String {
    let name = path.strip_prefix(ROOT).expect("found under the root");
    name.with_extension("").to_string_lossy().into_owned()
}

struct File {
    path: PathBuf,
    language: &'static Language,
    examples: Vec<Example>,
}

struct Example {
    /// The comment above it, if any.
    title: Option<String>,
    /// The lines of code, without their prefix.
    lines: Vec<String>,
    claims: Vec<Claim>,
}

/// What a line of carets says about the line of code above it.
struct Claim {
    /// Where in the file it is written, counted from one.
    at: usize,
    /// Which line of the example's code, counted from zero.
    line: usize,
    /// The characters under the carets.
    columns: Range<usize>,
    /// What the query calls them; `None` for nothing.
    name: Option<String>,
}

/// Reads the file. Its form is checked here, its claims later.
fn read(path: &Path) -> Result<File> {
    let content = fs::read_to_string(path)?;
    let mut language = None;
    let mut examples = Vec::new();
    let mut comment: Option<String> = None;
    let mut current: Option<Example> = None;

    for (index, line) in content.lines().enumerate() {
        let at = index + 1;
        let line = line.trim_end();

        if line.is_empty() {
            examples.extend(current.take());
            comment = None;
        } else if let Some(name) = line.strip_prefix("%language ") {
            if language.is_some() || current.is_some() || !examples.is_empty() {
                bail!("line {at}: `%language` comes once, before the examples");
            }
            language = Some(
                language::by_name(name)
                    .with_context(|| format!("line {at}: no language is called `{name}`"))?,
            );
        } else if let Some(text) = line.strip_prefix('#') {
            if current.is_some() {
                bail!("line {at}: a comment inside an example");
            }
            comment.get_or_insert_with(|| text.trim().to_owned());
        } else if line == ">" || line.starts_with("> ") {
            let example = current.get_or_insert_with(|| Example {
                title: comment.take(),
                lines: Vec::new(),
                claims: Vec::new(),
            });
            example.lines.push(line.get(2..).unwrap_or("").to_owned());
        } else if line.trim_start().starts_with('^') {
            let Some(example) = current.as_mut() else {
                bail!("line {at}: carets with no code above them");
            };
            let indent = line.len() - line.trim_start().len();
            let Some(start) = indent.checked_sub(2) else {
                bail!("line {at}: the carets stand left of the code");
            };
            let rest = line.trim_start();
            let carets = rest.len() - rest.trim_start_matches('^').len();
            let name = match rest[carets..].trim() {
                "" => bail!("line {at}: the carets name nothing; `-` for no token"),
                "-" => None,
                name => Some(name.to_owned()),
            };
            example.claims.push(Claim {
                at,
                line: example.lines.len() - 1,
                columns: start..start + carets,
                name,
            });
        } else {
            bail!("line {at}: neither code (`> `), carets, a comment nor a blank line");
        }
    }
    examples.extend(current);

    Ok(File {
        path: path.to_owned(),
        language: language.context("no `%language` line")?,
        examples,
    })
}

/// Every claim of the file that does not hold, each as it is to be shown.
fn check(file: &File) -> Vec<String> {
    let path = file
        .path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(&file.path);
    let mut failures = Vec::new();
    for example in &file.examples {
        let mut code = example.lines.join("\n");
        code.push('\n');
        let mut builder = TextBuilder::default();
        builder.append(&code);
        let text = builder.finish();
        let syntax = Syntax::new(file.language, &text).expect("the grammar is in this build");
        let tokens = syntax.tokens(&text, 0..text.len());

        let title = match &example.title {
            Some(title) => format!(" ({title})"),
            None => String::new(),
        };
        for claim in &example.claims {
            let line = &example.lines[claim.line];
            let Err(error) = holds(claim, line, text.line_start(claim.line), &tokens) else {
                continue;
            };
            failures.push(format!(
                "  {}:{}{title}: {error}\n    {line}\n    {}{}",
                path.display(),
                claim.at,
                " ".repeat(claim.columns.start),
                "^".repeat(claim.columns.len()),
            ));
        }
    }
    failures
}

/// Whether every character under the claim's carets is named as it says.
/// `start` is where its line begins in the example's text.
fn holds(claim: &Claim, line: &str, start: usize, tokens: &[Token<'_>]) -> Result<(), String> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    if claim.columns.end > chars.len() {
        return Err("the carets run past the end of the line".to_owned());
    }
    // The carets count characters and line up by columns, which is the
    // same only while every character is one column wide: a tab is as wide
    // as the editor makes it, a CJK letter or an emoji two.
    let wide = chars[..claim.columns.end]
        .iter()
        .find(|&&(_, c)| c == '\t' || c.width() != Some(1));
    if let Some((_, c)) = wide {
        return Err(format!(
            "{c:?} before the carets is not one column wide: what they point at is not known"
        ));
    }
    for column in claim.columns.clone() {
        let found = name_at(tokens, start + chars[column].0);
        if found != claim.name.as_deref() {
            return Err(format!(
                "column {column}: expected {}, found {}",
                claim.name.as_deref().unwrap_or("-"),
                found.unwrap_or("-"),
            ));
        }
    }
    Ok(())
}

/// What the character at `byte` is called. Tokens nest and an inner one
/// comes after the one around it, so the last that covers the byte is the
/// innermost — the one the screen shows (`editor::view`, `LineHighlight`).
fn name_at<'a>(tokens: &[Token<'a>], byte: usize) -> Option<&'a str> {
    tokens
        .iter()
        .rev()
        .find(|token| token.range.contains(&byte))
        .map(|token| token.name)
}
