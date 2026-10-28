# Ctrl+L selects the line the cursor is on, whole, with its line break, so
# the head lands at the start of the next line; pressed again it takes the
# next line too, and on the last line it ends at the end of the text. Each
# cursor takes its own line. Ctrl+A selects everything, from any number of
# cursors to one selection.

size 30 6
paste "one\ntwo\nthree"
key Ctrl+Home
key Right
key Ctrl+L

--- the first line, break included: the head is at the start of the second
0|1 one
 |  ~~~~
1|2 two
 |  ^
2|3 three
3-4|
5| …ch]* · Text · 3 lines · 2:1

key Ctrl+L

--- the second line joins the selection
1|2 two
 |  ~~~~
2|3 three
 |  ^
5| …ch]* · Text · 3 lines · 3:1

key Ctrl+L

--- the last line has no break to take: the selection ends with the text
2|3 three
 |  ~~~~~^
5| …ch]* · Text · 3 lines · 3:6

key Esc
key Ctrl+Home
key Shift+Alt+Down
key Shift+Alt+Down
key Ctrl+L

--- three cursors, three lines: each took its own
1|2 two
 |  ^~~~
2|3 three
 |  ^~~~~^
5| 3 cursors …t · 3 lines · 2:1

key Ctrl+A

--- Ctrl+A: one selection over everything, the cursor at the end
1|2 two
 |  ~~~~
2|3 three
 |  ~~~~~^
5| …ch]* · Text · 3 lines · 3:6

