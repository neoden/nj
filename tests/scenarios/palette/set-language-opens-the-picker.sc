# A scratch buffer has no file name to tell its language from, so it is
# plain text until told otherwise. "Set language" in the palette opens a
# second list, of the languages known, filtered and chosen the same way.
# The choice shows in the status bar and takes effect at once: the text is
# coloured by the language's grammar, and the comment that Ctrl+/ puts in
# is the chosen language's. The colours are a layer of their own, asked
# for with `---ink`.

%grammar Rust
size 40 16
paste "fn main() {}"
key Ctrl+Shift+P
wait
type "lang"

--- the palette narrowed to the one command
 0|1 fn main() {}
  |--------------^-------------------------
 1-10|
11|────────────────────────────────────────
12|   Set language
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| > lang
  |       ^
14|────────────────────────────────────────
15| 1/25 …cratch]* · Text · 1 lines · 1:13

key Enter
wait

--- the language list in its place, the palette gone
 2|────────────────────────────────────────
 3|   JSON
 4|   SCSS
 5|   HTML
 6|   Python
 7|   Rust
 8|   C++
 9|   C
10|   Go
11|   CSS
12|   TSX
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| >
  |   ^
15| 22/22 …ratch]* · Text · 1 lines · 1:13

type "ru"

--- narrowed to Rust
 2-10|
11|────────────────────────────────────────
12|   Rust
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| > ru
  |     ^
15| 1/22 …cratch]* · Text · 1 lines · 1:13

key Enter

--- the status bar names the language
11-14|
15|     [scratch]* · Rust · 1 lines · 1:13

---ink and the text is coloured as Rust
 0|1 fn main() {}
  |  kk ffff
 1-14|
15|     [scratch]* · Rust · 1 lines · 1:13

key Ctrl+/

--- the comment is Rust's
 0|1 // fn main() {}
  |-----------------^----------------------
15|     [scratch]* · Rust · 1 lines · 1:16

---ink the whole line is a comment now
 0|1 // fn main() {}
  |  ccccccccccccccc
15|     [scratch]* · Rust · 1 lines · 1:16

