# The status bar is nailed to the last row and the text keeps at least one
# row, so a panel gets what is left between them, which is not always what
# it asked for. Cut down, it keeps its rules and loses lines from its end.
# Too short for even its two rules and a line, it is not drawn at all — but
# it is still there: the keyboard is its, and an answer typed into the
# dark is taken.

size 40 10
paste "one"
key Ctrl+Q

--- room for the whole question
0|1 one
 |::---^----------------------------------
1-2|
3|────────────────────────────────────────
4| Unsaved changes
5|   s     Save and quit
6|   q     Quit anyway
7|   esc   Cancel
8|────────────────────────────────────────
9|      [scratch]* · Text · 1 lines · 1:4

size 40 7

--- cut down: the last line goes, the text keeps a row, the status its place
0|1 one
 |::---^----------------------------------
1|────────────────────────────────────────
2| Unsaved changes
3|   s     Save and quit
4|   q     Quit anyway
5|────────────────────────────────────────
6|      [scratch]* · Text · 1 lines · 1:4

size 40 4

--- too short for a panel: none is drawn
0|1 one
 |::---^----------------------------------
1-2|
3|      [scratch]* · Text · 1 lines · 1:4

type "q"

--- the question was still up, and the answer quits
exit 0

