# Enter on a line of nothing but indent empties what it leaves behind
# (`enter-keeps-the-indent`). Only the first cursor on a line may do that:
# a second cursor on the same line finds it already dealt with and leaves
# its own line alone, or it would empty the line the first cursor's indent
# just moved to. Here two cursors stand on a line of four spaces, after the
# second and after the fourth.

size 30 5
type "    "
key Home
key Shift+Right
key Shift+Right
key Ctrl+Shift+L

--- two spaces selected, and every other two: two selections on the line
0|1
 |  ~~^~^
1-3|
4| 2 cursors …t · 1 lines · 1:3

key Right

--- collapsed: a cursor after the second space and one after the fourth
0|1
 |    ^ ^

key Enter
key End

--- Enter at both: the old line is emptied once, and the two new lines keep four spaces each
0|1
1|2
 |      ^
2|3
 |      ^
4| 2 cursors …t · 3 lines · 2:5


key Up
key Up
key End

--- the old line is empty: End finds nothing, and both cursors meet there
0|1
 |::^---------------------------
1|2
2|3
4| …ch]* · Text · 3 lines · 1:1

