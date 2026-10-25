# Enter in the replace form replaces nothing by itself: the form closes,
# every hit is marked, and a question opens at the first hit after the
# cursor, selected, with the replacement in it and the keys that answer it.
# Esc closes the question, having replaced nothing.

size 72 7
paste "cat dog\ncat\ndog cat"
key Ctrl+Home
key Ctrl+H
type "cat"
key Tab
type "bird"
key Enter

--- the form is gone, every `cat` is marked, the first is selected and asked about
0|1 cat dog
 |  ~~~^
1|2 cat
 |  ===
2|3 dog cat
 |      ===
3|────────────────────────────────────────────────────────────────────────
4| Replace with "bird"?   y Replace   n Skip   a Replace all   esc Stop
5|────────────────────────────────────────────────────────────────────────
6| 1/3 "cat"                            [scratch]* · Text · 3 lines · 1:4

key Esc

--- Esc: the question is closed, nothing was replaced, the marks are gone
1|2 cat
2|3 dog cat
3-5|
6| Replaced 0                           [scratch]* · Text · 3 lines · 1:4

