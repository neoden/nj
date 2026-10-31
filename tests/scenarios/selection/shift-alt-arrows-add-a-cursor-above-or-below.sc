# Shift+Alt+Down and Shift+Alt+Up grow and shrink a column of cursors around
# the one placed by hand. Down adds a cursor on the row below the lowest,
# in the same column; a shorter line pulls its cursor in to its end, but
# the column is remembered. Up then takes the lowest one away again before
# it ever adds above, and the other way round: the column grows from the
# primary in one direction, and the opposite key shrinks it back. The
# primary stays what it was: the status shows it, and Esc comes back to
# it. At the edge of the text there is nowhere to add, and nothing happens.

size 30 7
paste "ab\nabcdef\nabcdef\nabcdef"
key Ctrl+Home
key Down
key End
key Shift+Alt+Down

--- a cursor on the row below, in the same column
0|1 ab
1|2 abcdef
 |        ^
2|3 abcdef
 |        ^
3|4 abcdef
4-5|
6| 2 cursors …t · 4 lines · 2:7

key Shift+Alt+Down

--- another below; the status still shows the first, the primary
3|4 abcdef
 |        ^
6| 3 cursors …t · 4 lines · 2:7

key Shift+Alt+Down

--- nothing below the last line: nothing happens
same

key Shift+Alt+Up

--- Up takes the lowest cursor away rather than adding above
3|4 abcdef
6| 2 cursors …t · 4 lines · 2:7

key Shift+Alt+Up

--- and the next: only the primary is left
1|2 abcdef
 |--------^---------------------
2|3 abcdef
6| …ch]* · Text · 4 lines · 2:7

key Shift+Alt+Up

--- now Up adds above: on the short line, pulled in to its end
0|1 ab
 |    ^
1|2 abcdef
 |        ^
6| 2 cursors …t · 4 lines · 2:7

key Shift+Alt+Up

--- nothing above the first line: nothing happens
same

key Shift+Alt+Down

--- Down takes the cursor above away rather than adding below
0|1 ab
1|2 abcdef
 |--------^---------------------
6| …ch]* · Text · 4 lines · 2:7

key Esc

--- a single cursor: Esc has nothing to drop
same

