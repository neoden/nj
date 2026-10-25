# Undo puts the cursors back as well as the text: every cursor returns to
# where it was before the edit, with the selection it had, however the
# cursors were moved or dropped in between. Redo puts them where the edit
# left them.

size 30 7
paste "one\ntwo\nthree"
key Ctrl+Home
key Ctrl+Alt+Down
key Ctrl+Alt+Down
key Shift+End

--- three cursors with their lines selected
0|1 one
 |  ~~~^
1|2 two
 |  ~~~^
2|3 three
 |  ~~~~~^
3-5|
6| 3 cursors…xt · 3 lines · 1:4

type "x"

--- each line replaced by `x`; three cursors after them
0|1 x
 |   ^
1|2 x
 |   ^
2|3 x
 |   ^
6| 3 cursors…xt · 3 lines · 1:2

key Esc
key Ctrl+End

--- the cursors dropped and the one left moved to the end
0|1 x
1|2 x
2|3 x
 |::-^--------------------------
6| …ch]* · Text · 3 lines · 3:2

key Ctrl+Z

--- undo: the text is back, and so are all three selections, the primary where it was
0|1 one
 |  ~~~^
1|2 two
 |  ~~~^
2|3 three
 |  ~~~~~^
6| 3 cursors…xt · 3 lines · 1:4

key Ctrl+Y

--- redo: the replacement again, and the three cursors after it
0|1 x
 |   ^
1|2 x
 |   ^
2|3 x
 |   ^
6| 3 cursors…xt · 3 lines · 1:2

