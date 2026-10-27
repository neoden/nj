# Ctrl+Shift+L is Ctrl+D all at once. On a bare cursor it selects the word
# under it; pressed again, or pressed on a selection, it puts a cursor on
# every occurrence of the selected text at once: whole words when the word
# was taken by the key, the exact text wherever it occurs when the
# selection was made by hand. The cursor placed by hand stays the primary.

size 30 7
paste "cat scatter\ncat\ncat"
key Ctrl+Home
key Down
key Ctrl+Shift+L

--- first press: the word under the cursor
0|1 cat scatter
1|2 cat
 |  ~~~^
2|3 cat
3-5|
6| …ch]* · Text · 3 lines · 2:4

key Ctrl+Shift+L

--- second press: every whole `cat`, three cursors; `scatter` is passed over, the primary stays on the second line
0|1 cat scatter
 |  ~~~^
2|3 cat
 |  ~~~^
6| 3 cursors …t · 3 lines · 2:4

key Esc
key Esc
key Ctrl+Home
key Shift+Right
key Shift+Right
key Shift+Right
key Ctrl+Shift+L

--- from a selection made by hand: every `cat`, inside `scatter` too, four cursors
0|1 cat scatter
 |  ~~~^ ~~~^
6| 4 cursors …t · 3 lines · 1:4

