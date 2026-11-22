# A row that ends short of the width — a wide character did not fit and
# went to the next row — is still a row to stop on. Down from a cell
# further right than the row reaches lands on its last letter, not on the
# wide character a row below; Up comes back the same way. The cell the
# cursor started in is remembered throughout.

size 12 6
paste "012345678\nabcdefghi日x"
key Ctrl+Home
key Alt+z
key End

--- the cursor after `8`, in the tenth cell; the row under it has nine letters
0|1 012345678
 |-----------^
1|2 abcdefghi
2|  日x
3-4|
5| …es · 1:10

key Down

--- Down: before `i`, the last letter of that row
0|1 012345678
1|2 abcdefghi
 |----------^-
2|  日x
 |------------
5| …nes · 2:9

key Down

--- Down: the row of the wide character is shorter still, so the end of the line
1|2 abcdefghi
 |------------
2|  日x
 |-----^------
5| …es · 2:13

key Up

--- Up: before `i` again
1|2 abcdefghi
 |----------^-
2|  日x
 |------------
5| …nes · 2:9

key Up

--- Up: after `8`, where it started
0|1 012345678
 |-----------^
1|2 abcdefghi
2|  日x
5| …es · 1:10

