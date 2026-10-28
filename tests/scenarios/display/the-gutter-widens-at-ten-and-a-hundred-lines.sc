# The gutter is as wide as the largest line number plus a space, so it
# grows by a column when the text reaches ten lines and again at a
# hundred, and gives the column back when the lines go away.

size 24 6
paste "a\nb\nc\nd\ne\nf\ng\nh\ni"

--- nine lines: one digit and a space
0|5 e
1|6 f
2|7 g
3|8 h
4|9 i
 |::-^--------------------
5| … Text · 9 lines · 9:2

key Enter

--- the tenth line: two digits, the old ones right-aligned under them
0| 6 f
1| 7 g
2| 8 h
3| 9 i
4|10
 |:::^--------------------
5| …ext · 10 lines · 10:1

paste "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n"

--- the hundredth line: three digits
0| 96
1| 97
2| 98
3| 99
4|100
 |::::^-------------------
5| …t · 100 lines · 100:1

key Ctrl+Z
key Ctrl+Z

--- undone back to nine lines, the gutter is one digit again
0|5 e
1|6 f
2|7 g
3|8 h
4|9 i
 |::-^--------------------
5| … Text · 9 lines · 9:2

