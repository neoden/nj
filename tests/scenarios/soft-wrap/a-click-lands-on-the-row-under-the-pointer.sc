# A click in a wrapped line puts the cursor on the row under the pointer,
# not on the first row of the line. The row that holds only the end of
# the line takes a click anywhere on it as a click at the end.

size 12 6
paste "abcdefghijklmnopqrst\nx"
key Ctrl+Home
key Alt+z

click 5 1

--- a click on the second row, fourth cell: before `n`
0|1 abcdefghij
 |------------
1|  klmnopqrst
 |-----^------
2|
 |------------
3|2 x
4|
5| …es · 1:14

click 8 2

--- a click on the third row, which is empty: the end of the line
1|  klmnopqrst
 |------------
2|
 |--^---------
5| …es · 1:21

click 5 3

--- a click past `x`: the end of the second line
0|1 abcdefghij
1|  klmnopqrst
2|
3|2 x
 |---^--------
5| …nes · 2:2

