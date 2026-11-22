# The column Up and Down remember is a column of the line, so it is still
# the right one when soft wrap is turned off between two of them. Down
# inside a wrapped line takes the cursor from column 7 to column 17, one
# row lower; with the wrap off, the next Down goes to column 17 of the
# next line, straight under it.

size 12 7
paste "abcdefghijklmnopqrstuvwxyz\nABCDEFGHIJKLMNOPQRSTUVWXYZ"
key Ctrl+Home
key Alt+z
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Down

--- Down from `h`: before `r`, the same cell of the next row, ten columns on in the line
0|1 abcdefghij
 |------------
1|  klmnopqrst
 |---------^--
2|  uvwxyz
 |------------
3|2 ABCDEFGHIJ
4|  KLMNOPQRST
5|  UVWXYZ
6| …es · 1:18

key Alt+z

--- Alt+Z: unwrapped; the cursor is still before `r`
0|1 ijklmnopqr
 |-----------^
1|2 IJKLMNOPQR
2-5|

key Down

--- Down: before `R`, under it
0|1 ijklmnopqr
1|2 IJKLMNOPQR
 |-----------^
6| …es · 2:18

