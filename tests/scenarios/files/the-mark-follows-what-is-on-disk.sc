# The `*` in the status means the text differs from the file, not that
# something was typed since the save: undo back to what was saved clears
# it, undo past the save brings it back, redo clears it again. A save also
# ends the typing group: a word carried on after the save is its own edit,
# and one undo takes back only that.

%file notes.txt ""
%start notes.txt
size 40 4
type "wor"
key Ctrl+S
?file notes.txt "wor"

--- saved: no mark
0|1 wor
 |::---^----------------------------------
1-2|
3|       notes.txt · Text · 1 lines · 1:4

type "d"

--- the same word carried on after the save: marked
0|1 word
 |::----^---------------------------------
3|      notes.txt* · Text · 1 lines · 1:5

key Ctrl+Z

--- one undo takes back only what came after the save: clean again
0|1 wor
 |::---^----------------------------------
3|       notes.txt · Text · 1 lines · 1:4

key Ctrl+Z

--- undone past the save: the text differs from the file, marked
0|1
 |::^-------------------------------------
3|      notes.txt* · Text · 1 lines · 1:1

key Ctrl+Y

--- redone to what was saved: clean
0|1 wor
 |::---^----------------------------------
3|       notes.txt · Text · 1 lines · 1:4

type "d"
key Ctrl+Z

--- a new edit undone: back to what was saved, clean
same

