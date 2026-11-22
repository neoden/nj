# With soft wrap, a line that grows to the right edge takes another row as
# it is typed, and the lines under it move down; deleting gives the row
# back. The row appears when the line fills the width exactly, since the
# cursor after the last letter needs a cell to stand in.

size 12 6
paste "abcdefgh\nnext"
key Ctrl+Home
key Alt+z
key End
type "i"

--- nine letters: one row, the cursor in the last cell
0|1 abcdefghi
 |-----------^
1|2 next
2-4|
5| …es · 1:10

type "j"

--- the tenth fills the row, and the cursor goes to a row of its own
0|1 abcdefghij
 |------------
1|
 |--^---------
2|2 next
5| …es · 1:11

type "k"

--- the eleventh is typed on that row
1|  k
 |---^--------
5| …es · 1:12

key Backspace
key Backspace

--- two Backspace: one row again, and the next line is back under it
0|1 abcdefghi
 |-----------^
1|2 next
2|
5| …es · 1:10

key Ctrl+Backspace

--- Ctrl+Backspace: the line is empty
0|1
 |--^---------
5| …nes · 1:1

