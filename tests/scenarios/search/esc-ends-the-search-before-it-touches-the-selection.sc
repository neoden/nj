# Esc does one thing at a time. While a search goes on, the first Esc ends
# it: the marks go, the selection on the hit stays. The next Esc collapses
# the selection to a cursor. A command that puts the hits to use, such as
# Ctrl+Shift+L, ends the search itself, so after it Esc is for the cursors
# the search left: the extra ones first, then the selection.

size 30 6
paste "a cat\na hat\na bat"
key Ctrl+Home
key Ctrl+F
type "at"
key Enter

--- the search is on, the first hit selected
0|1 a cat
 |     ~~^
1|2 a hat
 |     ==
2|3 a bat
 |     ==
3-4|
5| 1/3 "at"…ext · 3 lines · 1:6

key Esc

--- the marks are gone, the selection stays
1|2 a hat
2|3 a bat
5| …ch]* · Text · 3 lines · 1:6

key Esc

--- the selection collapses to a cursor
0|1 a cat
 |::-----^----------------------

key Ctrl+F
type "at"
key Enter
key Ctrl+Shift+L

--- every hit became a cursor, and that ended the search
0|1 a cat
 |     ~~^
1|2 a hat
 |     ~~^
2|3 a bat
 |     ~~^
5| 3 cursors…xt · 3 lines · 2:6

key Esc

--- the first Esc drops the extra cursors; the primary keeps its selection
0|1 a cat
2|3 a bat
5| …ch]* · Text · 3 lines · 2:6

key Esc

--- the second collapses the selection
1|2 a hat
 |::-----^----------------------

key Esc

--- a third Esc has nothing left to do
same

