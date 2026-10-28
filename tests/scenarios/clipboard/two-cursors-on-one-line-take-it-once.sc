# With nothing selected, Ctrl+C and Ctrl+X take whole lines, one per line
# under a cursor: two cursors on the same line take it once, two on
# different lines take a line each. A line pasted back goes in once per
# line too, not once per cursor.

size 30 7
paste "one one\ntwo\nthree"
key Ctrl+Home
key Ctrl+D
key Ctrl+D
key Left
key Ctrl+C
?clipboard "one one\n"

--- two cursors on the first line, nothing selected: the line is copied once
0|1 one one
 |  ^   ^
1|2 two
2|3 three
3-5|
6| 2 cursors …t · 3 lines · 1:1

key Ctrl+V

--- and pasted once, above, both cursors still on their text
0|1 one one
1|2 one one
 |  ^   ^
2|3 two
3|4 three
6| 2 cursors …t · 4 lines · 2:1

key Ctrl+X
?clipboard "one one\n"

--- cut once: one line gone, not two; the cursors land on the same spot and are one
1|2 two
 |::^---------------------------
2|3 three
3|
6| …ch]* · Text · 3 lines · 2:1

key Shift+Alt+Down
key Ctrl+X
?clipboard "two\nthree\n"

--- a cursor added below and cut: a line each. The break after `one one` is not taken, so an empty last line is left, as VS Code leaves it
1|2
 |::^---------------------------
2|
6| …ch]* · Text · 2 lines · 2:1

