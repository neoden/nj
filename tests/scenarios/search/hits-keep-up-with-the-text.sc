# A search going on is not a snapshot: as the text changes, hits appear
# where the query is typed in and go where it is broken, and the count in
# the status follows. The hit under the cursor stays current while the
# cursor is on it.

size 30 6
paste "ab ab\nab"
key Ctrl+Home
key Ctrl+F
type "ab"
key Enter

--- three hits, the first one selected
0|1 ab ab
 |  ~~^==
1|2 ab
 |  ==
2-4|
5| 1/3 "ab"…ext · 2 lines · 1:3

key End
type " ab"

--- a fourth hit was typed in; the count goes up
0|1 ab ab ab
 |::==-==-==^-------------------
5| 4 "ab"… Text · 2 lines · 1:9

key Ctrl+Home
key Delete

--- the first hit is broken: its `b` alone is no hit
0|1 b ab ab
 |::^-==-==---------------------
5| 3 "ab"… Text · 2 lines · 1:1

key Ctrl+End
key Backspace

--- the last hit is broken from the other end
0|1 b ab ab
 |    == ==
1|2 a
 |::-^--------------------------
5| 2 "ab"… Text · 2 lines · 2:2

