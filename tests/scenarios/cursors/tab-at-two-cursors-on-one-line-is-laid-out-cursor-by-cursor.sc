# Tab at a bare cursor pads to the next tab stop, and a cursor's column
# depends on what the cursors before it on the line have inserted — not
# by adding their lengths, but by laying the line out afresh, since a tab
# character between them stretches. Here `a`, a tab, `b`, with a cursor
# after `a` and one after the tab, at column 4 where `b` is. The first
# inserts three spaces to reach column 4; the tab now starts there and
# reaches 8; `b` is on 8 and the second cursor needs a whole level, not
# the one space that column 4 plus three would call for.

size 30 4
paste "a\tb"
key Home
key Right
click 6 0 Alt

--- a cursor after `a` and one before `b`, which sits past the tab at column 4
0|1 a   b
 |   ^  ^
1-2|
3| 2 cursors …t · 1 lines · 1:2

key Tab

--- both padded: `a`, three spaces, the tab to 8, four spaces, `b` at 12
0|1 a           b
 |      ^       ^
3| 2 cursors …t · 1 lines · 1:5

