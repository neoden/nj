# Whatever moves the cursor out of the view scrolls the view after it: an
# arrow at the edge, a jump, a search hit, an edit. The view keeps two rows
# of margin between the cursor and its edge where the text allows, so the
# cursor never sits on the very last row while there is more text below.

size 20 6
paste "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12"
key Ctrl+Home
key Down
key Down

--- line 3: two rows above it, the view has not moved
0| 1 1
1| 2 2
2| 3 3
 |---^----------------
3| 4 4
4| 5 5
5| … · 12 lines · 3:1

key Down

--- line 4 would leave only one row below; the view scrolls a line to keep two
0| 2 2
1| 3 3
2| 4 4
 |---^----------------
3| 5 5
4| 6 6
5| … · 12 lines · 4:1

key Ctrl+End
key Up
key Up

--- from the end, Up twice: line 10 is two rows from the top, the view holds
0| 8 8
1| 9 9
2|10 10
 |-----^--------------
3|11 11
4|12 12
5| …· 12 lines · 10:3

key Up

--- line 9 would be one row from the top: the view scrolls up a line to keep two
0| 7 7
1| 8 8
2| 9 9
 |----^---------------
3|10 10
4|11 11
5| … · 12 lines · 9:2

key Ctrl+F
type "3"
key Enter

--- the only hit is on line 3, far above: the view jumps up to it
0| 1 1
1| 2 2
2| 3 3
 |   ~^
3| 4 4
4| 5 5
5| 1/1 "3" …nes · 3:2

key Ctrl+G
type "11"
key Enter

--- a jump to line 11: the view goes to the bottom; the margin gives way where the text ends
0| 8 8
1| 9 9
2|10 10
3|11 11
 |---^----------------
4|12 12
5| 1 "3" …ines · 11:1

