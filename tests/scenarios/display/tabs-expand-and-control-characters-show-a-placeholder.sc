# A tab is as wide as it takes to reach the next stop, four columns apart,
# so one tab from column 0 is four cells and one from column 1 is three.
# A control character has no glyph; it is drawn as the replacement
# character in one cell, so it can be seen and stepped over. The bell in the last line below is the
# character itself, byte 7, since neither `type` nor an escape can write
# one.

size 24 5
paste "a\tb\n\tx\nabc"

--- a tab from column 1 reaches column 4, one from column 0 the same; the bell is a placeholder
0|1 a   b
1|2     x
2|3 ab�c
 |------^-----------------
3|
4| … Text · 3 lines · 3:5

key Ctrl+Home
key Right

--- on the tab: the caret marks the column, the tab's first cell
0|1 a   b
 |---^--------------------
2|3 ab�c
4| … Text · 3 lines · 1:2

key Right

--- after it: one keystroke, three cells
0|1 a   b
 |------^-----------------
4| … Text · 3 lines · 1:5

key Ctrl+End
key Left
key Left

--- on the bell: one cell, one step
0|1 a   b
2|3 ab�c
 |----^-------------------
4| … Text · 3 lines · 3:3

