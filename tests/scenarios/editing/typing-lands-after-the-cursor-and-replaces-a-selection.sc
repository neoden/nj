# Plain typing: each keystroke goes in at the cursor and the cursor follows.
# With something selected, the first keystroke replaces it. Enter splits the
# line, and the dirty mark appears with the first change.

size 30 5
type "hllo"

--- the text is after the gutter, the cursor after the text, the buffer dirty
0|1 hllo
 |::----^-----------------------
1-3|
4| …ch]* · Text · 1 lines · 1:5

key Home
key Right
type "e"

--- typed in the middle: the rest of the line moves right
0|1 hello
 |::--^-------------------------
4| …ch]* · Text · 1 lines · 1:3

key Home
key Shift+End
type "bye"

--- the selected line is replaced by what was typed
0|1 bye
 |::---^------------------------
4| …ch]* · Text · 1 lines · 1:4

key Enter
type "now"

--- Enter splits the line and typing goes on in the new one
0|1 bye
1|2 now
 |::---^------------------------
4| …ch]* · Text · 2 lines · 2:4


key Ctrl+K
key Ctrl+Shift+K
key Alt+K

--- a combination nothing is bound to inserts nothing and opens nothing
same


key Shift+Home
key Enter

--- Enter over a selection replaces it with the line break: `now` is gone, a line split in its place
1|2
2|3
 |::^---------------------------
4| …ch]* · Text · 3 lines · 3:1

