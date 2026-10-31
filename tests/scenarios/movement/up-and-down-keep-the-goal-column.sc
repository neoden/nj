# Up and Down remember the column the cursor set out from. Crossing a
# shorter line, the cursor is pulled in to its end, but on the next long
# line it is back where it was. Moving sideways sets a new goal. Columns are
# display columns: a wide character counts for two.

size 30 7
paste "abcdefgh\nab\n\nab日本cdef\nabcdefgh"
key Ctrl+Home
key End

--- at column nine on the first line
0|1 abcdefgh
 |----------^-------------------
1|2 ab
2|3
3|4 ab日本cdef
4|5 abcdefgh
5|
6| …ch]* · Text · 5 lines · 1:9

key Down

--- the second line is shorter: the cursor is at its end
0|1 abcdefgh
1|2 ab
 |----^-------------------------
6| …ch]* · Text · 5 lines · 2:3

key Down

--- the empty line: column one
1|2 ab
2|3
 |--^---------------------------
6| …ch]* · Text · 5 lines · 3:1

key Down

--- column nine again, which here is after six characters: each wide one took two columns
2|3
3|4 ab日本cdef
 |----------^-------------------
6| …ch]* · Text · 5 lines · 4:9

key Down

--- back on a long line, the cursor is at column nine again
3|4 ab日本cdef
4|5 abcdefgh
 |----------^-------------------
6| …ch]* · Text · 5 lines · 5:9

key Left
key Up
key Up
key Up
key Up

--- Left set a new goal; four Up lands on the first line at column eight
0|1 abcdefgh
 |---------^--------------------
4|5 abcdefgh
6| …ch]* · Text · 5 lines · 1:8

