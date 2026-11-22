# A cursor added with Shift+Alt+Down on a line too short for the column
# stands at the end of that line and remembers the column: 17 here. The
# first cursor is in column 17 too, which on the screen is the eighth cell
# of a second row. Down takes the added cursor a row down to the eighth
# cell, under the first, and not to column 17 of the next line, which is a
# row further. Up brings both back, and neither is stuck.

size 12 8
paste "abcdefghijklmnopqrs\nxy\nABCDEFGHIJKLMNOPQRSTUVWXY"
key Ctrl+Home
key Alt+z
key Down
key Right
key Right
key Right
key Right
key Right
key Right
key Right

--- the cursor before `r`: the eighth cell of its row, column 17 of its line
0|1 abcdefghij
 |------------
1|  klmnopqrs
 |---------^--
2|2 xy
3|3 ABCDEFGHIJ
4|  KLMNOPQRST
5|  UVWXY
6|
7| …es · 1:18

key Shift+Alt+Down

--- a second cursor on the next line, which is too short for column 17: it stands at the end of `xy`
0|1 abcdefghij
1|  klmnopqrs
 |         ^
2|2 xy
 |    ^
7| 2 cu …1:18

key Down

--- Down: the first cursor is at the end of `xy`, where the second was; the second is before `H`, a row down, in the eighth cell
1|  klmnopqrs
3|3 ABCDEFGHIJ
 |         ^
7| 2 cu … 2:3

key Up

--- Up: both are back: before `r`, and at the end of `xy`
1|  klmnopqrs
 |         ^
3|3 ABCDEFGHIJ
7| 2 cu …1:18

key Up

--- Up: the first before `h`, the second before `r`
0|1 abcdefghij
 |         ^
2|2 xy
7| 2 cu … 1:8

