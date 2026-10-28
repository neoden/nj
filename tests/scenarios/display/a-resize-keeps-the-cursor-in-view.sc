# The screen changing size is not a motion, but the cursor must still be
# on the screen afterwards: a view that shrinks scrolls so that the
# cursor's line and column stay in it, down or sideways. A view that grows
# fills its rows with text while there is any, so it never shows blank
# rows below the end while there are lines above the top.

size 30 8
paste "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten"

--- ten lines on seven rows: the cursor on the last line, the first lines scrolled off
0| 4 four
1| 5 five
2| 6 six
3| 7 seven
4| 8 eight
5| 9 nine
6|10 ten
 |:::---^-----------------------
7| …]* · Text · 10 lines · 10:4

size 30 4

--- three rows: the view scrolls down so the cursor's line is still on it
0| 8 eight
1| 9 nine
2|10 ten
 |:::---^-----------------------
3| …]* · Text · 10 lines · 10:4

size 30 8

--- seven rows again: nothing below to show, so the earlier lines come back
0| 4 four
1| 5 five
2| 6 six
3| 7 seven
4| 8 eight
5| 9 nine
6|10 ten
 |:::---^-----------------------
7| …]* · Text · 10 lines · 10:4

key Ctrl+Home
size 30 4

--- at the top, shrinking takes rows from the bottom and the cursor stays
0| 1 one
 |:::^--------------------------
1| 2 two
2| 3 three
3| …h]* · Text · 10 lines · 1:1

key Ctrl+End
key End
paste " abcdefghijklmnopqrs"

--- the last line made longer; it still fits in the width, so no scrolling
0| 8 eight
1| 9 nine
2|10 ten abcdefghijklmnopqrs
 |:::-----------------------^---
3| …* · Text · 10 lines · 10:24

size 16 4

--- narrower: the view scrolls sideways so the cursor's column is on it
0| 8
1| 9
2|10 hijklmnopqrs
 |:::------------^
3| …lines · 10:24

