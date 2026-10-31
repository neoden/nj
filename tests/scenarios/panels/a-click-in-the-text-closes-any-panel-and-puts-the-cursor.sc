# A click in the text takes the keyboard back there, and whatever panel is
# up gives way to it as it would to Esc: a field, or a question. The click
# is then a click, and the cursor lands where it was. A click on the panel
# itself is nobody's: the panel stays, the cursor too.

size 40 10
paste "alpha\nbeta"
key Ctrl+Home
key Ctrl+F
type "al"

--- the search field is open, the cursor still at the start
0|1 alpha
 |--^-------------------------------------
1|2 beta
2-5|
6|────────────────────────────────────────
7| Find: al
 |         ^
8|────────────────────────────────────────
9|      [scratch]* · Text · 2 lines · 1:1

click 5 1

--- the click closed the field and put the cursor in the text
0|1 alpha
1|2 beta
 |-----^----------------------------------
6-8|
9|      [scratch]* · Text · 2 lines · 2:4

key Ctrl+Q

--- a dirty buffer: the quit asks
3|────────────────────────────────────────
4| Unsaved changes
5|   s     Save and quit
6|   q     Quit anyway
7|   esc   Cancel
8|────────────────────────────────────────

click 10 5

--- a click on the question itself changes nothing
same

click 4 0

--- a click in the text takes the question back and puts the cursor
0|1 alpha
 |----^-----------------------------------
1|2 beta
3-8|
9|      [scratch]* · Text · 2 lines · 1:3

