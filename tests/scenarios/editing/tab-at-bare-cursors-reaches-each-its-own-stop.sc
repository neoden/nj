# Tab at a bare cursor pads to the next tab stop, and every cursor pads to
# its own: how much each inserts depends on the column it is in. The
# spaces it inserts after text are text, not indent, so Backspace there
# takes one character, not a level.

size 30 5
paste "one\ntwo"
key Ctrl+Home
key Ctrl+Right
click 3 1 Alt

--- cursors after `one` at column 3 and after `t` at column 1
0|1 one
 |     ^
1|2 two
 |   ^
2-3|
4| 2 cursors …t · 2 lines · 1:4

key Tab

--- each reaches its own next stop: one space after `one`, three after `t`
0|1 one
 |      ^
1|2 t   wo
 |      ^
4| 2 cursors …t · 2 lines · 1:5

key Backspace

--- Backspace after text takes one character, not a level
0|1 one
 |     ^
1|2 t  wo
 |     ^
4| 2 cursors …t · 2 lines · 1:4

