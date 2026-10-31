# Esc ends a search but does not forget it. With no search going on, F3
# starts the last one again from the cursor, marks come back, and the walk
# goes on from where the cursor is now, not from where it was.

size 30 6
paste "x one\ny one\nz one"
key Ctrl+Home
key Ctrl+F
type "one"
key Enter
key Esc

--- the search is over, the hit it found stays selected
0|1 x one
 |    ~~~^
1|2 y one
2|3 z one
3-4|
5| …ch]* · Text · 3 lines · 1:6

key Down
key Home

--- the cursor has moved to the second line
0|1 x one
1|2 y one
 |--^---------------------------
5| …ch]* · Text · 3 lines · 2:1

key F3

--- F3 searches for the same query again, from here: the second hit
0|1 x one
 |    ===
1|2 y one
 |    ~~~^
2|3 z one
 |    ===
5| 2/3 "one" …t · 3 lines · 2:6

key Shift+F3
key Shift+F3

--- and the search is on as before: Shift+F3 walks back and wraps
1|2 y one
 |    ===
2|3 z one
 |    ~~~^
5| 3/3 "one" …t · 3 lines · 3:6

