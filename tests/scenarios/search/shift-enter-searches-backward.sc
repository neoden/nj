# Shift+Enter in the find field starts the search in the other direction:
# from the cursor towards the start of the text, wrapping at the top. Once
# the search is on, F3 and Shift+F3 keep their own directions.

size 30 6
paste "cat\ndog\ncat\ndog"
key Ctrl+Home
key Down
key Down
key End

--- the cursor is at the end of the third line
0|1 cat
1|2 dog
2|3 cat
 |-----^------------------------
3|4 dog
4|
5| …ch]* · Text · 4 lines · 3:4

key Ctrl+F
type "cat"
key Shift+Enter

--- Shift+Enter lands on the hit before the cursor, on the same line
0|1 cat
 |  ===
2|3 cat
 |  ~~~^
5| 2/2 "cat" …t · 4 lines · 3:4

key Shift+F3

--- Shift+F3 goes back to the first line
0|1 cat
 |  ~~~^
2|3 cat
 |  ===
5| 1/2 "cat" …t · 4 lines · 1:4

key Shift+F3

--- past the start it wraps to the last hit
0|1 cat
 |  ===
2|3 cat
 |  ~~~^
5| 2/2 "cat" …t · 4 lines · 3:4

key F3

--- F3 still goes forward and wraps to the first
0|1 cat
 |  ~~~^
2|3 cat
 |  ===
5| 1/2 "cat" …t · 4 lines · 1:4

