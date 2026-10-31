# Every cursor remembers its own goal column. Moving a column of cursors
# down through lines of different lengths, each is pulled in by a short
# line and comes back to its own column on the next long one. Two cursors
# pulled to the same end of a short line become one, and stay one; the
# merged cursor starts afresh from where they met, with no goal of either.

size 30 8
paste "abcdefgh\nabcdefgh\nab\nabcdefgh\nabcdefgh"
key Ctrl+Home
key Right
key Right
click 8 1 Alt

--- two cursors: column three on the first line, column seven on the second
0|1 abcdefgh
 |    ^
1|2 abcdefgh
 |        ^
2|3 ab
3|4 abcdefgh
4|5 abcdefgh
5-6|
7| 2 cursors …t · 5 lines · 1:3

key Down

--- both down: the first to column three, the second pulled in to the end of `ab`
0|1 abcdefgh
1|2 abcdefgh
 |    ^
2|3 ab
 |    ^
7| 2 cursors …t · 5 lines · 2:3

key Down

--- the second is back in column seven; the first was never pulled
1|2 abcdefgh
3|4 abcdefgh
 |        ^
7| 2 cursors …t · 5 lines · 3:3

key Down

--- the last line: both in their columns
2|3 ab
3|4 abcdefgh
 |    ^
4|5 abcdefgh
 |        ^
7| 2 cursors …t · 5 lines · 4:3

key Up
key Up
key Up

--- three Up: both are back where they started, each at its own goal
0|1 abcdefgh
 |    ^
1|2 abcdefgh
 |        ^
3|4 abcdefgh
4|5 abcdefgh
7| 2 cursors …t · 5 lines · 1:3

key Esc
key Down
key End
click 7 1 Alt

--- one line, two cursors: the primary at its end, another after `abcde`
0|1 abcdefgh
1|2 abcdefgh
 |       ^  ^
7| 2 cursors …t · 5 lines · 2:9

key Down

--- on the short line both are pulled in to the same end: one cursor
1|2 abcdefgh
2|3 ab
 |----^-------------------------
7| …ch]* · Text · 5 lines · 3:3

key Down

--- it stays one, and the merge forgot both goals: from here Down keeps column three
2|3 ab
3|4 abcdefgh
 |----^-------------------------
7| …ch]* · Text · 5 lines · 4:3

