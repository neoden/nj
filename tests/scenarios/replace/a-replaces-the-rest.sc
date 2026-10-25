# `a` answers for every hit not answered for yet: the one asked about and
# all the walk had still to come to, in one edit. Hits already skipped with
# `n` are left alone. The question closes, the status counts what was
# replaced, and one undo takes the whole lot back.

size 72 7
paste "cat dog\ncat\ndog cat\ncat"
key Ctrl+Home
key Ctrl+H
type "cat"
key Tab
type "bird"
key Enter
type "n"

--- the first hit skipped, the question at the second
0|1 cat dog
 |  ===
1|2 cat
 |  ~~~^
2|3 dog cat
 |      ===
3|────────────────────────────────────────────────────────────────────────
4| Replace with "bird"?   y Replace   n Skip   a Replace all   esc Stop
5|────────────────────────────────────────────────────────────────────────
6| 2/4 "cat"                            [scratch]* · Text · 4 lines · 2:4

type "a"

--- `a`: the second, third and fourth are replaced at once with a cursor each; the first stays `cat`
0|1 cat dog
1|2 bird
 |      ^
2|3 dog bird
 |          ^
3|4 bird
 |      ^
4-5|
6| Replaced 3                           [scratch]* · Text · 4 lines · 2:5

key Ctrl+Z

--- one undo brings all three back, selected under the three cursors
1|2 cat
 |  ~~~^
2|3 dog cat
 |      ~~~^
3|4 cat
 |  ~~~^
6| 3 cursors                            [scratch]* · Text · 4 lines · 2:4

