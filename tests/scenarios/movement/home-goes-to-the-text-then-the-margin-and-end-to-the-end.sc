# Home goes to the first non-blank character first and to column one
# second, and back to the text on a third press. End goes to the end of
# the line.

size 30 4
paste "  ab foo.bar"

--- the cursor at the end of the line, as the paste left it
0|1   ab foo.bar
 |--------------^---------------
1-2|
3| …h]* · Text · 1 lines · 1:13

key Home

--- Home lands on the first non-blank character
0|1   ab foo.bar
 |----^-------------------------
3| …ch]* · Text · 1 lines · 1:3

key Home

--- Home again lands on column one
0|1   ab foo.bar
 |--^---------------------------
3| …ch]* · Text · 1 lines · 1:1

key Home

--- and a third Home goes back to the text
0|1   ab foo.bar
 |----^-------------------------
3| …ch]* · Text · 1 lines · 1:3

key End

--- End goes to the end of the line
0|1   ab foo.bar
 |--------------^---------------
3| …h]* · Text · 1 lines · 1:13

