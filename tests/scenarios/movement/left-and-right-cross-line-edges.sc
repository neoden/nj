# Left and Right move by one character, and at the edge of a line they
# cross to the next or previous one: Right at the end of a line lands on
# the start of the next, Left at the start lands on the end of the
# previous.

size 30 4
paste "ab\ncd"
key Ctrl+Home
key End

--- at the end of the first line
0|1 ab
 |----^-------------------------
1|2 cd
2|
3| …ch]* · Text · 2 lines · 1:3

key Right

--- Right at the end crosses to the start of the next line
0|1 ab
1|2 cd
 |--^---------------------------
3| …ch]* · Text · 2 lines · 2:1

key Left

--- Left at the start crosses back to the end of the previous one
0|1 ab
 |----^-------------------------
1|2 cd
3| …ch]* · Text · 2 lines · 1:3

