# Ctrl+X is a copy and a removal in one: what was selected is on the
# clipboard and gone from the text, the cursor where it stood. One undo
# brings the text back with its selection; the clipboard keeps the copy, so
# the cut text can still be pasted after the undo. With several cursors
# every one cuts its own piece.

size 30 6
paste "one two three\nfour five"
key Ctrl+Home
key Ctrl+Right
key Shift+Ctrl+Right
key Ctrl+X

--- ` two` cut: gone from the text, the cursor where it was
0|1 one three
 |::---^------------------------
1|2 four five
2-4|
5| …ch]* · Text · 2 lines · 1:4

key Ctrl+Z

--- undo: the text and the selection are back
0|1 one two three
 |     ~~~~^
5| …ch]* · Text · 2 lines · 1:8

key End
key Ctrl+V

--- the clipboard still holds the cut text: pasted at the end
0|1 one two three two
 |::-----------------^----------
5| …h]* · Text · 2 lines · 1:18

key Ctrl+Home
key Ctrl+Alt+Down
key Shift+Ctrl+Right
key Ctrl+X

--- a word selected on each line and cut at once, a piece each; both cursors stay
0|1  two three two
 |  ^
1|2  five
 |  ^
5| 2 cursors …t · 2 lines · 1:1

key Ctrl+Z

--- one undo restores both
0|1 one two three two
 |  ~~~^
1|2 four five
 |  ~~~~^
5| 2 cursors …t · 2 lines · 1:4

