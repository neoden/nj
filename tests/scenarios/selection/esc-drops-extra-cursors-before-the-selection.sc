# Esc undoes the selection state one layer at a time. With several cursors,
# the first Esc keeps only the primary one, selection and all; the next
# collapses its selection to a cursor; and a bare cursor is left alone.
# Esc never touches the text.

size 30 6
paste "one\ntwo\nthree"
key Ctrl+Home
key Shift+Alt+Down
key Shift+Alt+Down
key Shift+End

--- three cursors, each with its line selected
0|1 one
 |  ~~~^
1|2 two
 |  ~~~^
2|3 three
 |  ~~~~~^
3-4|
5| 3 cursors …t · 3 lines · 1:4

key Esc

--- the first Esc drops the added cursors; the one placed by hand, on the first line, keeps its selection
1|2 two
2|3 three
5| …ch]* · Text · 3 lines · 1:4

key Esc

--- the second collapses the selection to a cursor
0|1 one
 |-----^------------------------

key Esc

--- a third Esc has nothing left to do
same

