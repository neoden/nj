# Ctrl+G asks for a line number and Enter takes the cursor to the start of
# that line. What is not a number is refused in the status bar, and a number
# past the end lands on the last line, both without moving anything else.

size 30 8
paste "one\ntwo\nthree\nfour\nfive"
key Ctrl+Home
key Ctrl+G
type "3"
key Enter

--- the cursor is at the start of line 3
0|1 one
1|2 two
2|3 three
 |--^---------------------------
3|4 four
4|5 five
5-6|
7| …ch]* · Text · 5 lines · 3:1

key Ctrl+G
type "later"
key Enter

--- not a number: the status says so and the cursor stays
7| Not a line nu …5 lines · 3:1

key Ctrl+G
type "99"
key Enter

--- past the end: the last line
2|3 three
4|5 five
 |--^---------------------------
7| …ch]* · Text · 5 lines · 5:1

