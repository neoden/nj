# Ctrl+Up and Ctrl+Down scroll the view by a line, the wheel by three, and
# neither moves the cursor: it may scroll off the screen and stays where it
# was in the text. The view stops at the top, and at the bottom when the
# last line is on the last row. The next cursor movement brings the view
# back to the cursor.

size 20 6
paste "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12"
key Ctrl+Home

--- the cursor on line 1, at the top
0| 1 1
 |:::^----------------
1| 2 2
2| 3 3
3| 4 4
4| 5 5
5| … · 12 lines · 1:1

key Ctrl+Down

--- Ctrl+Down: the view moved down a line; the cursor is off the screen, still at 1:1
0| 2 2
1| 3 3
2| 4 4
3| 5 5
4| 6 6

scroll down 5 0

--- the wheel moves three lines at once
0| 5 5
1| 6 6
2| 7 7
3| 8 8
4| 9 9

scroll down 5 0
scroll down 5 0

--- the view stops with the last line on the last row
0| 8 8
1| 9 9
2|10 10
3|11 11
4|12 12

key Ctrl+Up

--- Ctrl+Up: one line back
0| 7 7
1| 8 8
2| 9 9
3|10 10
4|11 11

scroll up 5 0
scroll up 5 0
scroll up 5 0

--- the wheel back up stops at the top
0| 1 1
 |:::^----------------
1| 2 2
2| 3 3
3| 4 4
4| 5 5

key Ctrl+Down
key Ctrl+Down
key Ctrl+Down
key Right

--- the view was three lines down; a cursor movement brings it back to the cursor
0| 1 1
 |:::-^---------------
5| … · 12 lines · 1:2


key Ctrl+End
key Ctrl+A

--- Ctrl+A from the end: everything selected, and the view stays where it was
0| 8 8
 |   ~~
1| 9 9
 |   ~~
2|10 10
 |   ~~~
3|11 11
 |   ~~~
4|12 12
 |   ~~^
5| …· 12 lines · 12:3

