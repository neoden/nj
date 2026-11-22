# Shift+Alt+Down adds a cursor on the next line, not on the next row, even
# where the line the cursor is on has rows under it: the cursors are a
# column in the text. With soft wrap turned off again they are under one
# another.

size 12 10
paste "abcdefghijklmnopqrst\nABCDEFGHIJKLMNOPQRST\n01234567890123456789"
key Ctrl+Home
key Alt+z
key Down
key Right
key Right
key Right

--- the cursor before `n`, on the second row of the first line
0|1 abcdefghij
 |------------
1|  klmnopqrst
 |-----^------
2|
 |------------
3|2 ABCDEFGHIJ
4|  KLMNOPQRST
5|
6|3 0123456789
7|  0123456789
8|
9| …es · 1:14

key Shift+Alt+Down

--- a second cursor before `N`: the same place in the next line, three rows down
0|1 abcdefghij
1|  klmnopqrst
 |     ^
2|
4|  KLMNOPQRST
 |     ^
9| 2 cu …1:14

key Shift+Alt+Down

--- a third in the third line, three rows down again
7|  0123456789
 |     ^
9| 3 cu …1:14

key Alt+z

--- Alt+Z: unwrapped, the three cursors are one under the other
0|1 efghijklmn
 |           ^
1|2 EFGHIJKLMN
 |           ^
2|3 4567890123
 |           ^
3-4|
6-7|

