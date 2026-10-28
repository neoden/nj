# While the question is open, `y` replaces the hit it stands on and moves
# to the next, `n` leaves it and moves on, and Esc stops the walk where it
# is: what was replaced stays replaced, the rest is left alone, and the
# status says how many were replaced. After the last hit the question
# closes by itself. Nothing else is taken as an answer. The form remembers
# the last query and replacement, so a stopped walk is picked up with Enter.

size 72 7
paste "cat dog\ncat\ndog cat\ncat"
key Ctrl+Home
key Ctrl+H
type "cat"
key Tab
type "bird"
key Enter

--- four hits, the question at the first
0|1 cat dog
 |  ~~~^
1|2 cat
 |  ===
2|3 dog cat
 |      ===
3|────────────────────────────────────────────────────────────────────────
4| Replace with "bird"?   y Replace   n Skip   a Replace all   esc Stop
5|────────────────────────────────────────────────────────────────────────
6| 1/4 "cat"                            [scratch]* · Text · 4 lines · 1:4

type "y"

--- `y`: the first is `bird`, the question moved to the second; three left
0|1 bird dog
1|2 cat
 |  ~~~^
6| 1/3 "cat"                            [scratch]* · Text · 4 lines · 2:4

type "n"

--- `n`: the second stays `cat`, the question moved to the third
0|2 cat
 |  ===
1|3 dog cat
 |      ~~~^
2|4 cat
 |  ===
6| 2/3 "cat"                            [scratch]* · Text · 4 lines · 3:8

type "x"

--- a letter that is no answer does nothing
same

key Esc

--- Esc stops: one replaced, two untouched, the search is over
0|1 bird dog
1|2 cat
2|3 dog cat
 |      ~~~^
3|4 cat
4-5|
6| Replaced 1                           [scratch]* · Text · 4 lines · 3:8

key Ctrl+Home
key Ctrl+H
key Enter

--- opened again, the form remembers both the query and the replacement: the walk starts over the three left
1|2 cat
 |  ~~~^
2|3 dog cat
 |      ===
3|────────────────────────────────────────────────────────────────────────
4| Replace with "bird"?   y Replace   n Skip   a Replace all   esc Stop
5|────────────────────────────────────────────────────────────────────────
6| 1/3 "cat"                            [scratch]* · Text · 4 lines · 2:4

type "y"
type "y"
type "y"

--- `y` to the last hit: the question closed by itself, three replaced
1|2 bird
2|3 dog bird
3|4 bird
 |::----^-----------------------------------------------------------------
4-5|
6| Replaced 3                           [scratch]* · Text · 4 lines · 4:5


key Ctrl+A
paste "a a"
key Ctrl+H
key Ctrl+A
type "a"
key Tab
key Ctrl+A
type "aa"
key Enter
type "y"

--- a replacement that contains the query: the first hit replaced, the question at the second, not at the `a`s just written
0|1 aa a
 |  == ~^
1-2|
3|────────────────────────────────────────────────────────────────────────
4| Replace with "aa"?   y Replace   n Skip   a Replace all   esc Stop
5|────────────────────────────────────────────────────────────────────────
6| 3/3 "a"                              [scratch]* · Text · 1 lines · 1:5

type "y"

--- the second replaced, and the walk is over: what was written is not offered to itself
0|1 aa aa
 |::-----^----------------------------------------------------------------
3-5|
6| Replaced 2                           [scratch]* · Text · 1 lines · 1:6

