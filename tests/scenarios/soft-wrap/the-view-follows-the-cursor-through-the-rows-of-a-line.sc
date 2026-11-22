# With soft wrap, the view scrolls by rows, not by lines: a line taller
# than the screen is scrolled through, so that the row the cursor is on
# shows, with the margin over and under it where there are rows to show.
# Page Down goes by rows too. The line here takes nine rows: eight of ten
# letters, and one for its end.

size 12 5
paste "aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeeeffffffffffgggggggggghhhhhhhhhh\nlast"
key Ctrl+Home
key Alt+z

--- the first four rows of the line; the rest is under the screen
0|1 aaaaaaaaaa
 |--^---------
1|  bbbbbbbbbb
 |------------
2|  cccccccccc
 |------------
3|  dddddddddd
 |------------
4| …nes · 1:1

key End

--- End: the end of the line, with the next line under it as the margin
0|  gggggggggg
 |------------
1|  hhhhhhhhhh
 |------------
2|
 |--^---------
3|2 last
4| …es · 1:81

key Up
key Up

--- two Up: the cursor is on what was the top row, and one row is scrolled in over it
0|  ffffffffff
 |------------
1|  gggggggggg
 |--^---------
2|  hhhhhhhhhh
 |------------
3|
 |------------
4| …es · 1:61

key Up

--- one more: and one row again
0|  eeeeeeeeee
 |------------
1|  ffffffffff
 |--^---------
2|  gggggggggg
 |------------
3|  hhhhhhhhhh
 |------------
4| …es · 1:51

key Home

--- Home: the start of the line, five rows up
0|1 aaaaaaaaaa
 |--^---------
1|  bbbbbbbbbb
 |------------
2|  cccccccccc
 |------------
3|  dddddddddd
 |------------
4| …nes · 1:1

key PageDown

--- Page Down: three rows down the same line
0|  bbbbbbbbbb
 |------------
1|  cccccccccc
 |------------
2|  dddddddddd
 |--^---------
3|  eeeeeeeeee
 |------------
4| …es · 1:31

