# A file opened at a place shows it in the middle of the screen, top to
# bottom, with the lines around it on both sides, rather than at the bottom
# edge where scrolling down from the top would leave it. Near either end of
# the text there is less to show on that side, and no empty space instead.
# The screen is the one the editor starts with: 40 by 10, nine rows of text.

%shell seq 1 40 > long.txt
%start long.txt:20

--- line 20 on the middle row, four lines above it and four below
0|16 16
1|17 17
2|18 18
3|19 19
4|20 20
 |---^------------------------------------
5|21 21
6|22 22
7|23 23
8|24 24
9|      long.txt · Text · 41 lines · 20:1

%reset
%shell seq 1 40 > long.txt
%start long.txt:20-22

--- lines 20 to 22 in the middle, three lines above them and three below
0|17 17
1|18 18
2|19 19
3|20 20
 |   ^~~
4|21 21
 |   ~~~
5|22 22
 |   ~~~
6|23 23
7|24 24
8|25 25
9|      long.txt · Text · 41 lines · 20:1

%reset
%shell seq 1 40 > long.txt
%start long.txt:10-30

--- taller than the screen: from line 10, under the margin
0| 8 8
1| 9 9
2|10 10
 |   ^~~
3|11 11
 |   ~~~
4|12 12
 |   ~~~
5|13 13
 |   ~~~
6|14 14
 |   ~~~
7|15 15
 |   ~~~
8|16 16
 |   ~~~
9|      long.txt · Text · 41 lines · 10:1

%reset
%shell seq 1 40 > long.txt
%start long.txt:2

--- near the top: the text from its first line
0| 1 1
1| 2 2
 |---^------------------------------------
2| 3 3
3| 4 4
4| 5 5
5| 6 6
6| 7 7
7| 8 8
8| 9 9
9|       long.txt · Text · 41 lines · 2:1

%reset
%shell seq 1 40 > long.txt
%start long.txt:39

--- near the end: the last line at the bottom
0|33 33
1|34 34
2|35 35
3|36 36
4|37 37
5|38 38
6|39 39
 |---^------------------------------------
7|40 40
8|41
9|      long.txt · Text · 41 lines · 39:1

