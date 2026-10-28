# Replacing `a` with `aa` writes new `a`s into the text. They light up as
# hits, since the search is live, but the walk does not stop at them: the
# question is asked only about the hits that were there when it started.

size 72 7
paste "a a"
key Ctrl+H
type "a"
key Tab
type "aa"
key Enter
type "y"

--- the first hit replaced: the question at the second, not at the `a`s just written
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

