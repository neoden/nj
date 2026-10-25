# The question starts at the hit the cursor is on. Found with a search and
# selected, a hit is asked about first when the replace form opens over it,
# not skipped for the one after it, as F3 would skip it.

size 72 7
paste "cat dog\ncat\ndog cat"
key Ctrl+Home
key Ctrl+F
type "cat"
key Enter
key F3
key F3

--- the last `cat` is selected by the search
0|1 cat dog
 |  ===
1|2 cat
 |  ===
2|3 dog cat
 |      ~~~^
3-5|
6| 3/3 "cat"                            [scratch]* · Text · 3 lines · 3:8

key Ctrl+H
key Tab
type "bird"
key Enter

--- Enter asks about that very hit; the selection does not move
3|────────────────────────────────────────────────────────────────────────
4| Replace with "bird"?   y Replace   n Skip   a Replace all   esc Stop
5|────────────────────────────────────────────────────────────────────────

