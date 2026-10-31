# A typing group is letters of one class typed in a row. A line break is
# not a letter: it is a group of its own, between the word before and the
# word after. Typing after deleting starts a new group too, so an undo
# takes back what was typed and leaves the deletion.

size 30 5
type "ab"
key Enter
type "cd"

--- a word, a line break, a word
0|1 ab
1|2 cd
 |----^-------------------------
2-3|
4| …ch]* · Text · 2 lines · 2:3

key Ctrl+Z

--- the break is a group of its own: the second word goes, the break stays
1|2
 |--^---------------------------
4| …ch]* · Text · 2 lines · 2:1

key Ctrl+Z

--- then the break
0|1 ab
 |----^-------------------------
1|
4| …ch]* · Text · 1 lines · 1:3

key Backspace
type "c"

--- a deletion, then typing
0|1 ac
 |----^-------------------------

key Ctrl+Z

--- typing after deleting is a new group: only the `c` goes, the `b` stays gone
0|1 a
 |---^--------------------------
4| …ch]* · Text · 1 lines · 1:2

