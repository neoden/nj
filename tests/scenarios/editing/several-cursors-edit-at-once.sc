# Every cursor edits at once, each at its own place. Typing, Backspace and
# Enter apply to all of them, and one undo takes back what all of them did.
# A cursor is added on the line above or below with Ctrl+Alt+Up or Down,
# at the same column, which here is the end of every line.

size 30 7
paste "one\ntwo\nsix"
key Ctrl+Home
key End
key Ctrl+Alt+Down
key Ctrl+Alt+Down

--- three cursors, one at the end of each line
0|1 one
 |     ^
1|2 two
 |     ^
2|3 six
 |     ^
3-5|
6| 3 cursors…xt · 3 lines · 1:4

type ";"

--- each line got the semicolon
0|1 one;
 |      ^
1|2 two;
 |      ^
2|3 six;
 |      ^
6| 3 cursors…xt · 3 lines · 1:5

key Backspace
key Backspace

--- each lost two characters
0|1 on
 |    ^
1|2 tw
 |    ^
2|3 si
 |    ^
6| 3 cursors…xt · 3 lines · 1:3

key Enter

--- Enter splits at every cursor: three new lines
0|1 on
1|2
 |  ^
2|3 tw
3|4
 |  ^
4|5 si
5|6
 |  ^
6| 3 cursors…xt · 6 lines · 2:1

key Ctrl+Z

--- one undo takes back all three splits
0|1 on
 |    ^
1|2 tw
 |    ^
2|3 si
 |    ^
3-5|
6| 3 cursors…xt · 3 lines · 1:3

key Esc

--- Esc drops the added cursors; the one placed by hand, on the first line, stays
0|1 on
 |::--^-------------------------
1|2 tw
2|3 si
6| …ch]* · Text · 3 lines · 1:3

