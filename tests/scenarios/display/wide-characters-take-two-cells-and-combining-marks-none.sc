# A cell is a grapheme cluster, not a character or a byte. An East Asian
# character is one cluster two cells wide, so the cursor steps over it in
# one keystroke and lands two columns on. A base letter with a combining
# accent after it is one cluster one cell wide. A combining mark with no
# base to lean on measures zero, so it is given a cell and drawn on a
# dotted circle, which is how it can be seen and stepped over.

size 24 5
paste "a日b\néx\ńz"

--- the end of the text; the lone accent on the last line has its own cell
0|1 a日b
1|2 éx
2|3 ◌́z
 |::--^-------------------
3|
4| … Text · 3 lines · 3:3

key Ctrl+Home
key Right

--- after the `a`, one cell in
0|1 a日b
 |::-^^-------------------
2|3 ◌́z
4| … Text · 3 lines · 1:2

key Right

--- after the wide character: one keystroke, two cells
0|1 a日b
 |::---^------------------
4| … Text · 3 lines · 1:4

key Right
key Down
key Home
key Right

--- after `e` with its accent: the base and the mark are one cell
0|1 a日b
1|2 éx
 |::-^--------------------
4| … Text · 3 lines · 2:2

key Down
key Home
key Right

--- after the lone accent: one cell, though it measures zero
1|2 éx
2|3 ◌́z
 |::-^--------------------
4| … Text · 3 lines · 3:2

