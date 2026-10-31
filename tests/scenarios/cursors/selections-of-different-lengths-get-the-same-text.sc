# Selections need not match: each is replaced by what is typed, whatever
# its length, and the text after each closes up by a different amount.
# Pasting is the one exception: a copy made with several cursors is kept
# in pieces, and pasted into as many cursors it goes back piece by piece;
# into any other number every cursor gets the whole of it.

size 30 7
paste "a bb ccc\nx y z"
key Ctrl+Home
click 4 0 Alt
click 7 0 Alt
key Shift+Ctrl+Right

--- a cursor at each word, then a word each: selections of one, two and three characters
0|1 a bb ccc
 |  ~^~~^~~~^
1|2 x y z
2-5|
6| 3 cursors …t · 2 lines · 1:2

type "-"

--- each replaced by one dash; the gaps closed by different amounts
0|1 - - -
 |   ^ ^ ^

key Ctrl+Z
key Ctrl+C
key Esc
key Down
key Home
click 4 1 Alt
click 6 1 Alt

--- undone and the three pieces copied; three bare cursors on the second line, one before each letter
0|1 a bb ccc
1|2 x y z
 |  ^ ^ ^
6| 3 cursors …t · 2 lines · 2:1

key Ctrl+V

--- as many cursors as pieces: each gets its own, in order
1|2 ax bby cccz
 |   ^   ^    ^
6| 3 cursors …t · 2 lines · 2:2

key Ctrl+Z
key Esc
key Ctrl+V

--- a single cursor gets all three pieces, joined with line breaks
1|2 a
2|3 bb
3|4 cccx y z
 |-----^------------------------
6| …ch]* · Text · 4 lines · 4:4

