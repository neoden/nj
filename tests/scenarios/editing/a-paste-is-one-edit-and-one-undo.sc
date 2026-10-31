# Text pasted from the terminal arrives whole, not as keystrokes: it goes
# in as one edit, even across lines, and one undo takes all of it back.
# Nothing about it is retyped, so no auto-indent gets in the way of what
# was pasted.

size 30 6
type "    start"
key Enter

--- a line indented by four, and a new one under it with the same indent
0|1     start
1|2
 |------^-----------------------
2-4|
5| …ch]* · Text · 2 lines · 2:5

paste "a\nb\nc"

--- three lines pasted at once: the first after the indent, the others as they came
1|2     a
2|3 b
3|4 c
 |---^--------------------------
5| …ch]* · Text · 4 lines · 4:2

key Ctrl+Z

--- one undo takes the whole paste back
1|2
 |------^-----------------------
2-3|
5| …ch]* · Text · 2 lines · 2:5

