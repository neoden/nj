# A tab that would reach past the right edge is not cut there: it starts
# the next row whole. It is as wide as it is in the line, where it runs
# from column nine to the stop at twelve, so the row starts with three
# blank cells.

size 12 5
paste "abcdefghi\tx"
key Ctrl+Home
key Alt+z

--- nine letters on the first row; the tab and `x` on the second
0|1 abcdefghi
 |--^---------
1|     x
 |------------
2-3|
4| …nes · 1:1

key End
key Left

--- the cursor before `x`, after the three cells of the tab
0|1 abcdefghi
 |------------
1|     x
 |-----^------
4| …es · 1:13

