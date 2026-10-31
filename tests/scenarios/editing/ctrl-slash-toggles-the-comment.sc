# Ctrl+/ comments the touched lines out with the language's marker, or
# uncomments them when every one of them is commented already. Markers line
# up with the shallowest line, so a block reads as a block. Blank lines are
# skipped either way. A language without a line comment, such as plain
# text, says so instead.

size 30 7
type "note"
key Ctrl+/

--- plain text has no line comment: a message, no change
0|1 note
 |------^-----------------------
1-5|
6| No line comme …1 lines · 1:5

%file main.rs "fn main() {\n    let x = 1;\n\n    let y = 2;\n}"
key Ctrl+O
type "main.rs"
key Enter
type "d"
wait
key Down

--- the scratch text discarded, a Rust file is open; the cursor is on the first indented line
0|1 fn main() {
1|2     let x = 1;
 |--^---------------------------
2|3
3|4     let y = 2;
4|5 }
6| …n.rs · Rust · 5 lines · 2:1

key Ctrl+/

--- one line commented, the marker at its own indent
1|2     // let x = 1;
 |--^---------------------------
6| ….rs* · Rust · 5 lines · 2:1

key Ctrl+/

--- the same key uncomments it
1|2     let x = 1;
 |--^---------------------------

key Up
key Shift+Down
key Shift+Down
key Shift+Down
key Shift+Down
key Ctrl+/

--- a block of mixed depth: every marker at the column of the shallowest line; the blank line is skipped
0|1 // fn main() {
 |  ~~~~~~~~~~~~~~~
1|2 //     let x = 1;
 |  ~~~~~~~~~~~~~~~~~~
2|3
 |  ~
3|4 //     let y = 2;
 |  ~~~~~~~~~~~~~~~~~~
4|5 }
 |  ^
6| ….rs* · Rust · 5 lines · 5:1

key Ctrl+/

--- all of them commented, so the same key uncomments them all
0|1 fn main() {
 |  ~~~~~~~~~~~~
1|2     let x = 1;
 |  ~~~~~~~~~~~~~~~
3|4     let y = 2;
 |  ~~~~~~~~~~~~~~~

