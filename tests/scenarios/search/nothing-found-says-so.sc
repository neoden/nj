# A search with no hit leaves the text and the cursor as they were and says
# so in the status bar. The search itself goes on with nothing to show:
# typing clears the message, and the status keeps saying so.

size 30 5
paste "one two"
key Ctrl+Home
key Ctrl+F
type "three"
key Enter

--- nothing selected, nothing marked, the status says not found
0|1 one two
 |--^---------------------------
1-3|
4| Not found …t · 1 lines · 1:1

key F3

--- F3 looks again and finds nothing again
same

type "x"

--- typing clears the message; the search is still on, with no hit
0|1 xone two
 |---^--------------------------
4| Not found "th …1 lines · 1:2

