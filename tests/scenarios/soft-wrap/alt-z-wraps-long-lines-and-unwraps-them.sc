# Alt+Z turns soft wrap on: a line wider than the view goes on on the rows
# under it, with no number in the gutter, and nothing is off to the side.
# Up and Down go by rows, Home and End by the whole line. Alt+Z again turns
# it off, and the view scrolls sideways to the cursor as before.

size 20 7
paste "abcdefghijklmnopqrstuvwxyz0123456789\nshort"
key Ctrl+Home

--- not wrapped: the line runs off the right edge
0|1 abcdefghijklmnopqr
 |--^-----------------
1|2 short
2-5|
6| …t · 2 lines · 1:1

key Alt+z

--- Alt+Z: the line fills two rows of eighteen columns, and its end takes a third
1|  stuvwxyz0123456789
 |--------------------
2|
 |--------------------
3|2 short

key Down

--- Down: to the second row of the same line, under where the cursor was
0|1 abcdefghijklmnopqr
 |--------------------
1|  stuvwxyz0123456789
 |--^-----------------
6| … · 2 lines · 1:19

key End

--- End: to the end of the line, on the third row, which has nothing else
1|  stuvwxyz0123456789
 |--------------------
2|
 |--^-----------------
6| … · 2 lines · 1:37

key Alt+z

--- Alt+Z again: one row a line, scrolled sideways to the cursor
0|1 tuvwxyz0123456789
 |-------------------^
1|2
2-3|

