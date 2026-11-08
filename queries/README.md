# Queries and their tests

One directory per language, named as `src/buffer/grammar.rs` links it.
Each holds:

- `highlights.scm` — the highlight query. The queries are kept here, not
  taken from the grammar crates, because the crates disagree on a basic
  rule: when two patterns name the same node, which one wins. Here the
  last one wins, as in tree-sitter's own highlighter. So general patterns
  go first and specific ones after. A pattern that differs from the
  crate's query has a comment starting with `Changed:` or `Added:`.
- `injections.scm` — which parts of the text another grammar parses, if
  any.
- `*.test` — examples of the language with the names the query must give
  them. Any file with that suffix is run; a language may have several.

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

- `%language NAME` — the language, by its name from `language.rs`. Once,
  before the examples.
- `# TEXT` — a comment. The first comment line above an example is its
  name in the output.
- `> CODE` — a line of code. `>` alone is an empty line. Several in a row
  make one example. The code starts after the two-column prefix.
- `^^^ NAME` — the query names every character above the carets NAME.
  `-` means no name: plain text. A line of code may have any number of
  caret lines. They line up with the code, two columns in.
- A blank line ends the example.

Each example is parsed on its own, as a whole file of the language. An
example that does not parse breaks nothing else. A fragment that needs a
root element around it has to include one. Injections apply: a
`<script>` in an HTML example is checked as JavaScript.

Where tokens nest, a character takes the innermost one, as on the
screen. That is why the string above is claimed in two parts: the escape
in the middle is not string.

The carets count characters but line up by columns. The two agree only
while every character before the carets is one column wide. A tab, a CJK
letter or an emoji before the carets fails the claim.

## Running

```sh
cargo test --test highlights                 # all of them
cargo test --test highlights -- rust/        # names containing `rust/`
cargo test --test highlights -- --code rust/ | nj --language Rust
                                             # the code alone, to see
```

A failure names the file and line, the column, and what the query said
instead. A language whose grammar is not in the build is skipped. Without
a filter, every grammar in the build must have examples here.
