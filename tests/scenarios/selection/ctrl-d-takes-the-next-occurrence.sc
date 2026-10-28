# Ctrl+D on a bare cursor selects the word under it. Every press after that
# adds a cursor on the next occurrence of the same word, whole words only,
# wrapping around the end of the text and passing over what is already
# selected; once every occurrence is taken, nothing more happens. The
# cursor placed by hand stays the primary, so Esc comes back to it. Started
# from a selection made by hand instead, Ctrl+D takes that text wherever it
# occurs, inside words too.

size 30 7
paste "cat scatter\ncat\ncat"
key Ctrl+Home
key Down
key Ctrl+D

--- the word under the cursor is selected
0|1 cat scatter
1|2 cat
 |  ~~~^
2|3 cat
3-5|
6| …ch]* · Text · 3 lines · 2:4

key Ctrl+D

--- the next whole `cat`, on the third line; `scatter` is passed over
2|3 cat
 |  ~~~^
6| 2 cursors …t · 3 lines · 2:4

key Ctrl+D

--- wrapped around to the first line
0|1 cat scatter
 |  ~~~^
6| 3 cursors …t · 3 lines · 2:4

key Ctrl+D

--- every whole `cat` is taken: nothing more is added
same

key Esc
key Esc

--- Esc twice: back to a bare cursor on the second line, where it was placed
0|1 cat scatter
1|2 cat
 |::---^------------------------
2|3 cat
6| …ch]* · Text · 3 lines · 2:4

key Ctrl+Home
key Shift+Right
key Shift+Right
key Shift+Right
key Ctrl+D

--- from a selection made by hand, `cat` is found inside `scatter` as well
0|1 cat scatter
 |  ~~~^ ~~~^
1|2 cat
6| 2 cursors …t · 3 lines · 1:4


key Ctrl+A
paste "cat"
key Ctrl+D

--- a cursor resting right after a word still finds it
0|1 cat
 |  ~~~^
1-2|
6| …ch]* · Text · 1 lines · 1:4

key Ctrl+A
paste "a.b c.d"
key Home
key Right
key Ctrl+D
key Ctrl+D

--- punctuation under the cursor is a word of its own: both dots, not the letters around them
0|1 a.b c.d
 |   ~^  ~^
6| 2 cursors …t · 1 lines · 1:3

