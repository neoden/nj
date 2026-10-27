# Line commands count lines, not cursors. Two cursors on one line are one
# line to Ctrl+L, which selects it once, and to Tab over a selection, which
# indents it once; two cursors on neighbouring lines select two lines that
# meet at the break and stay two selections. Shift+Tab takes one level off
# each touched line, however many cursors touch it.

size 30 7
paste "one two\nthree\nfour"
key Ctrl+Home
click 7 0 Alt

--- two cursors on the first line
0|1 one two
 |  ^    ^
1|2 three
2|3 four
3-5|
6| 2 cursors …t · 3 lines · 1:1

key Ctrl+L

--- Ctrl+L: the line is selected once; one selection, not two
0|1 one two
 |  ~~~~~~~~
1|2 three
 |  ^
6| …ch]* · Text · 3 lines · 2:1

key Tab

--- Tab over the selection indents the line once
0|1     one two
 |  ~~~~~~~~~~~~

key Shift+Tab

--- Shift+Tab takes the level back off, once
0|1 one two
 |  ~~~~~~~~

key Esc
key Ctrl+Home
key Ctrl+Alt+Down
key Ctrl+L

--- two cursors on two lines: Ctrl+L selects both, meeting at the break and staying two
1|2 three
 |  ^~~~~~
2|3 four
 |  ^
6| 2 cursors …t · 3 lines · 2:1

key Tab

--- Tab indents each line once; the selections stay two, each over its own line with its new indent
0|1     one two
 |  ~~~~~~~~~~~~
1|2     three
 |  ^~~~~~~~~~

