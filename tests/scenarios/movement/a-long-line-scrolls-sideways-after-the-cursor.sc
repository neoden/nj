# Lines are not wrapped yet: what does not fit the width is off the right
# edge, and the view scrolls sideways to keep the cursor in it, by as much
# as it has to and no more, in either direction. The gutter stays put; only
# the text moves, every line by the same amount, so a short line may show
# nothing at all.

size 20 5
paste "abcdefghijklmnopqrstuvwxyz\nshort"
key Ctrl+Home

--- the line runs off the right edge; the cursor at its start
0|1 abcdefghijklmnopqr
 |::^-----------------
1|2 short
2-3|
4| …t · 2 lines · 1:1

key End

--- End: the view has scrolled so that the cursor is in the last column
0|1 jklmnopqrstuvwxyz
 |::-----------------^
1|2
4| … · 2 lines · 1:27

key Down

--- Down pulls the cursor to the end of the short line, and the view scrolls back only as far as that: column six, where the short line has nothing left to show
0|1 fghijklmnopqrstuvw
1|2
 |::^-----------------
4| …t · 2 lines · 2:6

key Home

--- Home scrolls back to column one
0|1 abcdefghijklmnopqr
1|2 short
 |::^-----------------
4| …t · 2 lines · 2:1

key Up
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Right

--- seventeen Right on the long line: the cursor is in the last column, nothing has scrolled
0|1 abcdefghijklmnopqr
 |::-----------------^
1|2 short
4| … · 2 lines · 1:18

key Right

--- one more and the view scrolls by exactly one column
0|1 bcdefghijklmnopqrs
 |::-----------------^
1|2 hort
4| … · 2 lines · 1:19

