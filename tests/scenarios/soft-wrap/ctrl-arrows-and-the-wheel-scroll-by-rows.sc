# With soft wrap, Ctrl+Down and Ctrl+Up scroll the view by a row and the
# wheel by three, through the rows of one line: the view may start in the
# middle of a line, and then the gutter has no number on its first row.
# Scrolling stops when the last row of the document is at the bottom.

size 12 5
paste "aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeeeffffffffffgggggggggghhhhhhhhhh\nlast"
key Ctrl+Home
key Alt+z

--- the first four rows of a line of nine
0|1 aaaaaaaaaa
 |--^---------
1|  bbbbbbbbbb
 |------------
2|  cccccccccc
 |------------
3|  dddddddddd
 |------------
4| …nes · 1:1

key Ctrl+Down

--- Ctrl+Down: one row down; the cursor is off the screen
0|  bbbbbbbbbb
 |------------
1|  cccccccccc
 |------------
2|  dddddddddd
 |------------
3|  eeeeeeeeee
 |------------

scroll down 5 0

--- the wheel: three rows more
0|  eeeeeeeeee
 |------------
1|  ffffffffff
 |------------
2|  gggggggggg
 |------------
3|  hhhhhhhhhh
 |------------

scroll down 5 0
scroll down 5 0

--- two more turns stop with the last line at the bottom
0|  gggggggggg
 |------------
1|  hhhhhhhhhh
 |------------
2|
 |------------
3|2 last

key Ctrl+Up

--- Ctrl+Up: one row back
0|  ffffffffff
 |------------
1|  gggggggggg
 |------------
2|  hhhhhhhhhh
 |------------
3|
 |------------

