# Enter starts the new line where the current one starts: with its
# indentation copied. Only the part of the indent before the cursor is
# copied; the rest stays with the text. What would stay behind on the old
# line, if it is nothing but indent, is dropped: no line is left holding a
# row of spaces, whether Enter was pressed at its end or inside it. The
# indent itself is not lost: it moves to the cursor's line. Blank space
# shows as nothing, so where it matters the cursor is sent to the end of
# the line to show how long it is.

size 30 7
type "    deep"

--- a line indented by four
0|1     deep
 |::--------^-------------------
1-5|
6| …ch]* · Text · 1 lines · 1:9

key Enter
type "same"

--- the new line starts under the first
0|1     deep
1|2     same
 |::--------^-------------------
6| …ch]* · Text · 2 lines · 2:9

key Home
key Left
key Left
key Enter

--- Enter inside the indent: two spaces go with the new line
1|2
2|3     same
 |::--^-------------------------
6| …ch]* · Text · 3 lines · 3:3

key Up
key End

--- and nothing stayed behind: the old line is empty
1|2
 |::^---------------------------
2|3     same
6| …ch]* · Text · 3 lines · 2:1

key Down
key End
key Enter

--- a new line after `same` starts with its four
1|2
3|4
 |::----^-----------------------
6| …ch]* · Text · 4 lines · 4:5

key Enter
key Up
key End

--- Enter on the indent-only line leaves it empty: End finds nothing
3|4
 |::^---------------------------
4|5
6| …ch]* · Text · 5 lines · 4:1

key Down
key End

--- the indent moved on to the next line, with the cursor
3|4
4|5
 |::----^-----------------------
6| …ch]* · Text · 5 lines · 5:5

