# A scratch buffer has no file name to tell its language from, so it is
# plain text until told otherwise. "Set language" in the palette opens a
# second list, of the languages known, filtered and chosen the same way.
# The choice shows in the status bar and takes effect at once: the comment
# that Ctrl+/ puts in is the chosen language's.

size 40 16
paste "fn main() {}"
key Ctrl+Shift+P
wait
type "lang"

--- the palette narrowed to the one command
 0|1 fn main() {}
  |::------------^-------------------------
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
 3|   YAML
 4|   TOML
 5|   JSON
 6|   HTML
 7|   Python
 8|   Rust
 9|   C++
10|   C
11|   Go
12|   CSS
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| >
  |   ^
15| 16/16 …ratch]* · Text · 1 lines · 1:13

type "ru"

--- narrowed to Rust
 2-10|
11|────────────────────────────────────────
12|   Rust
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| > ru
  |     ^
15| 1/16 …cratch]* · Text · 1 lines · 1:13

key Enter

--- the status bar names the language
11-14|
15|     [scratch]* · Rust · 1 lines · 1:13

key Ctrl+/

--- the comment is Rust's
 0|1 // fn main() {}
  |::---------------^----------------------
15|     [scratch]* · Rust · 1 lines · 1:16

