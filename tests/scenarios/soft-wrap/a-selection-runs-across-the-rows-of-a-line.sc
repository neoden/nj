# A selection in a wrapped line runs from row to row with nothing extra at
# the right edge: the cell that shows a selected line break is drawn once,
# after the end of the line, which here is on a row of its own because the
# line fills its two rows exactly.

size 12 6
paste "abcdefghijklmnopqrst\nx"
key Ctrl+Home
key Alt+z
key Right
key Right
key Right
key Right
key Right
key Right
key Right
key Shift+Down

--- Shift+Down from `h`: to the end of the first row and on to `r` on the second
0|1 abcdefghij
 |         ~~~
1|  klmnopqrst
 |  ~~~~~~~^
2|
3|2 x
4|
5| …es · 1:18

key Shift+Down

--- again: to the end of the line, on the third row; the line break is not selected
1|  klmnopqrst
 |  ~~~~~~~~~~
2|
 |  ^
5| …es · 1:21

key Shift+Down

--- again: over the line break, which shows as one cell on the third row
2|
 |  ~
3|2 x
 |  ~^
5| …nes · 2:2

