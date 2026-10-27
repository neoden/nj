# Another file over unsaved changes is asked about. `d` discards them and
# opens; `s` saves them first and then opens. Both are checked by opening
# the first file again afterwards.

size 40 9
%file mine.txt "mine"
%file theirs.txt "theirs"
key Ctrl+O
type "mine.txt"
key Enter
wait
key End
type "!"
key Ctrl+O
type "theirs.txt"
key Enter

--- the question, and the text still as it was
0|1 mine!
 |::-----^--------------------------------
1|
2|────────────────────────────────────────
3| Unsaved changes
4|   s     Save and open
5|   d     Discard and open
6|   esc   Cancel
7|────────────────────────────────────────
8|       mine.txt* · Text · 1 lines · 1:6

key Ctrl+D

--- a combination does not answer
same

type "d"

--- discarded: the other file is open
0|1 theirs
 |::^-------------------------------------
2-7|
8|      theirs.txt · Text · 1 lines · 1:1

key End
type "?"
key Ctrl+O
type "mine.txt"
key Enter
type "s"
wait

--- saved first: the first file is back, as it was on disk
0|1 mine
 |::^-------------------------------------
8|        mine.txt · Text · 1 lines · 1:1

key Ctrl+O
type "theirs.txt"
key Enter
wait

--- and the other one kept what was saved
0|1 theirs?
 |::^-------------------------------------
8|      theirs.txt · Text · 1 lines · 1:1

