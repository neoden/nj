# Two cursors that come to the same place become one, and selections that
# come to overlap become one selection; what is merged stays merged, and
# the primary survives the merge. Cursors meet by movement, and by edits
# that take away what was between them.

size 30 6
type "ab"
key Home
click 4 0 Alt

--- two cursors, one at each end of `ab`
0|1 ab
 |  ^ ^
1-4|
5| 2 cursors …t · 1 lines · 1:1

key Right

--- Right moves both: the first is now between the letters, the second had nowhere to go
0|1 ab
 |   ^^
5| 2 cursors …t · 1 lines · 1:2

key Right

--- the first catches up with the second: one cursor
0|1 ab
 |----^-------------------------
5| …ch]* · Text · 1 lines · 1:3

key Left
key Left
click 4 0 Alt
key Backspace

--- two cursors again; Backspace at each: the first had nothing before it, the second took `b`
0|1 a
 |  ^^
5| 2 cursors …t · 1 lines · 1:1

key Backspace

--- the second took `a` and landed on the first: one cursor
0|1
 |--^---------------------------
5| …ch]* · Text · 1 lines · 1:1

type "ab ab"
key Home
key Ctrl+D
key Ctrl+D

--- two selections, one on each `ab`, with the space between them
0|1 ab ab
 |  ~~^~~^
5| 2 cursors …t · 1 lines · 1:3

key Shift+Right

--- both heads move right: the first selection now touches the second
0|1 ab ab
 |  ~~~^~^
5| 2 cursors …t · 1 lines · 1:4

key Shift+Right

--- the first grows into the second: one selection over everything
0|1 ab ab
 |  ~~~~~^
5| …ch]* · Text · 1 lines · 1:6

