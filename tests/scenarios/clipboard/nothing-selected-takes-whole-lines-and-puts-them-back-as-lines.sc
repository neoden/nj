# Ctrl+C with nothing selected copies the whole line the cursor is on,
# break included, and Ctrl+V puts such a copy back as a line: above the
# line the cursor is on, whatever column the cursor is in, and the cursor
# stays on its own text, pushed down with it. Ctrl+X with nothing selected
# takes the line out the same way. A copy made from a selection goes in at
# the cursor instead, as text.

size 30 7
paste "alpha\nbeta\ngamma"
key Ctrl+Home
key Down
key Right
key Right
key Ctrl+C

--- the cursor inside `beta`, nothing selected: Ctrl+C took the whole line
0|1 alpha
1|2 beta
 |::--^-------------------------
2|3 gamma
3-5|
6| …ch]* · Text · 3 lines · 2:3

key Down
key End
key Ctrl+V

--- pasted above `gamma` as a line of its own, from any column; the cursor keeps its text
1|2 beta
2|3 beta
3|4 gamma
 |::-----^----------------------
6| …ch]* · Text · 4 lines · 4:6

key Ctrl+X

--- Ctrl+X with nothing selected takes `gamma` out whole; the last line has no break after it, so the one before it goes too, and the cursor lands at the end of the line above
2|3 beta
 |::----^-----------------------
3|
6| …ch]* · Text · 3 lines · 3:5

key Ctrl+Home
key Ctrl+V

--- the taken line is a line: pasted back above the first
0|1 gamma
1|2 alpha
 |::^---------------------------
2|3 beta
3|4 beta
6| …ch]* · Text · 4 lines · 2:1

key Right
key Shift+Right
key Ctrl+C
key Down
key Ctrl+V

--- copied from a selection, the paste goes in at the cursor as text, not as a line
1|2 alpha
2|3 belta
 |::---^------------------------
6| …ch]* · Text · 4 lines · 3:4

