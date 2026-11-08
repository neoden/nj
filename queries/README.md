# Queries and their tests

One directory per language, named as `src/buffer/grammar.rs` links it:

- `highlights.scm` — the highlight query. Written here, not taken from the
  grammar's crate: the crates do not agree on which of two patterns naming
  the same node wins. Here the last one does, as in tree-sitter's own
  highlighter, so general patterns come first and specific ones after. A
  pattern that departs from the crate's query says so in a comment
  starting with `Changed:` or `Added:`.
- `injections.scm` — which pieces of the text another grammar reads, if
  any.
- `*.test` — examples of the language and what the query must call them.
  Any name with that suffix counts; a language may have several files.

## The test format

```text
%language Rust

# Constants by name
> const MAX_SIZE: u8 = 1;
        ^^^^^^^^ constant
                  ^^ type.builtin

# A nested token wins: the escape is not string
> let s = "a\tb";
          ^^ string
            ^^ escape
              ^^ string
```

- `%language NAME` — the language, by its name from `language.rs`; once,
  before the examples.
- `# TEXT` — a comment. The first comment line above an example names it
  in the output.
- `> CODE` — a line of code; `>` alone is an empty line. Several in a row
  are one example. The code is what comes after the two-column prefix.
- `^^^ NAME` — every character above the carets is named NAME by the
  query. `-` for none: the text is plain. A line of code may have any
  number of these; they line up with the code, two columns in.
- A blank line ends the example.

Each example is parsed on its own, as a whole file of the language, so one
that does not parse spoils no other; a fragment that needs a root around
it must bring it along. Injections apply, so a `<script>` in an HTML
example is checked as JavaScript. Where tokens nest, a character takes
the innermost, as on the screen. The carets count characters and line up
by columns, which is the same only while every character before them is
one column wide: a tab, a CJK letter or an emoji before the carets fails
the claim.

## Running

```sh
cargo test --test highlights                 # all of them
cargo test --test highlights -- rust/        # names containing `rust/`
cargo test --test highlights -- --code rust/ | nj --language Rust
                                             # the code alone, to see
```

A failure names the file and line, the column, and what the query said
instead. A language whose grammar the build left out is skipped. Without a
filter, every grammar linked in must have examples here.
