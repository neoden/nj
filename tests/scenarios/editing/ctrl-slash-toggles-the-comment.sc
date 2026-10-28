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
 |::----^-----------------------
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
 |::^---------------------------
2|3
3|4     let y = 2;
4|5 }
6| …n.rs · Rust · 5 lines · 2:1

key Ctrl+/

--- one line commented, the marker at its own indent
1|2     // let x = 1;
 |::^---------------------------
6| ….rs* · Rust · 5 lines · 2:1

key Ctrl+/

--- the same key uncomments it
1|2     let x = 1;
 |::^---------------------------

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

%file mixed.rs "\tone();\n  two();"
key Ctrl+O
type "mixed.rs"
key Enter
type "d"
wait
key Ctrl+A
key Ctrl+/

--- mixed indents, a tab and two spaces: the marker goes at the shallowest column, before a tab it cannot split
0|1 //  one();
 |  ~~~~~~~~~~~
1|2   // two();
 |  ~~~~~~~~~~~^
2-4|
6| …rs* · Rust · 2 lines · 2:12

key Ctrl+/
key Ctrl+S
?file mixed.rs "\tone();\n  two();"

--- and comes off again, the tab intact
0|1     one();
 |  ~~~~~~~~~~~
1|2   two();
 |  ~~~~~~~~^
6| …d.rs · Rust · 2 lines · 2:9

