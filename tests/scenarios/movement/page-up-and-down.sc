# PageDown moves the cursor down by a screen of rows less one, PageUp up by
# the same, and the view follows the cursor keeping its margin of rows
# above and below where the text allows. Both keep the goal column. When
# fewer rows are left than a page, the cursor lands on the last line, or
# on the first going up, still in its column.

size 20 6
paste "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12"
key Ctrl+Home
key End

--- twelve lines, five rows of text; the cursor in column two of the first
0| 1 1
 |:::-^---------------
1| 2 2
2| 3 3
3| 4 4
4| 5 5
5| … · 12 lines · 1:2

key PageDown

--- down by four rows to line 5; the view keeps two rows of margin below the cursor
0| 3 3
1| 4 4
2| 5 5
 |:::-^---------------
3| 6 6
4| 7 7
5| … · 12 lines · 5:2

key PageDown

--- another four: line 9
0| 7 7
1| 8 8
2| 9 9
 |:::-^---------------
3|10 10
4|11 11
5| … · 12 lines · 9:2

key PageDown

--- fewer than four rows left: the last line, in the same column
0| 8 8
1| 9 9
2|10 10
3|11 11
4|12 12
 |:::-^---------------
5| …· 12 lines · 12:2

key PageUp

--- PageUp goes up by four rows, to line 8
0| 6 6
1| 7 7
2| 8 8
 |:::-^---------------
3| 9 9
4|10 10
5| … · 12 lines · 8:2

key PageUp
key PageUp

--- fewer than four rows left above: the first line, in the same column
0| 1 1
 |:::-^---------------
1| 2 2
2| 3 3
3| 4 4
4| 5 5
5| … · 12 lines · 1:2

