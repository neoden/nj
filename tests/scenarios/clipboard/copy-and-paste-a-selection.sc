# Ctrl+C copies what is selected and leaves it selected; Ctrl+V puts the
# copy in at the cursor, or in place of a selection, and the cursor lands
# after it. The copy stays for as many pastes as wanted. What is copied
# also goes to the desktop clipboard, so another program can paste it.

size 30 6
paste "one two three"
key Home
key Ctrl+Right
key Shift+Ctrl+Left
key Ctrl+C
?clipboard "one"

--- `one` is selected and copied; the selection stays
0|1 one two three
 |  ^~~
1-4|
5| …ch]* · Text · 1 lines · 1:1

key End
type " "
key Ctrl+V

--- pasted at the end, after a space; the cursor is after the paste
0|1 one two three one
 |::-----------------^----------
5| …h]* · Text · 1 lines · 1:18

key Ctrl+V

--- the copy is not spent: pasted again
0|1 one two three oneone
 |::--------------------^-------
5| …h]* · Text · 1 lines · 1:21

key Home
key Shift+Ctrl+Right
key Ctrl+V

--- pasted over a selection: `one` replaced by `one`, nothing else moved
0|1 one two three oneone
 |::---^------------------------
5| …ch]* · Text · 1 lines · 1:4

key Right
key Shift+Ctrl+Right
key Ctrl+V

--- over `two`: the selection is gone and the copy stands in its place
0|1 one one three oneone
 |::-------^--------------------
5| …ch]* · Text · 1 lines · 1:8

