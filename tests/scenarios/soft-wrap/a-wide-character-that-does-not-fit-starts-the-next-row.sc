# With soft wrap, a character two cells wide that has one cell left on its
# row is not cut in half: it starts the next row, and the cell it left is
# blank. That cell is part of the line all the same: it has the colour of
# the current line, and a click on it puts the cursor on its own row,
# before the last letter, not on the wide character a row below.

size 12 5
paste "abcdefghi日x\nshort"
key Ctrl+Home
key Alt+z

--- nine letters and a blank cell on the first row, the wide character on the second
0|1 abcdefghi
 |--^---------
1|  日x
 |------------
2|2 short
3|
4| …nes · 1:1

key End

--- End: the cursor after `x`, on the second row
0|1 abcdefghi
 |------------
1|  日x
 |-----^------
4| …es · 1:13

click 11 0

--- a click on the blank cell: before `i`, the last letter of that row
0|1 abcdefghi
 |----------^-
1|  日x
 |------------
4| …nes · 1:9

