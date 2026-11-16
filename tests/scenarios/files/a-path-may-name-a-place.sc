# `nj file:3` starts with the cursor on line 3, and `nj file:3:4` on its
# fourth character, the way a compiler writes a place. A column counts
# characters, so a tab is one. `nj file:2-4` starts with lines 2 to 4
# selected. Which part of the text is the name is settled by the files that
# are there: the longest name that is a file wins.

%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:3
size 30 8

--- the start of line 3
0|1 one
1|2 two
2|3     three
 |--^---------------------------
3|4 four
4|5 five
5-6|
7| ….txt · Text · 5 lines · 3:1

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:3:4
size 30 8

--- the fourth character of line 3: the tab is one, so the cursor is on `r`
0|1 one
1|2 two
2|3     three
 |--------^---------------------
3|4 four
4|5 five
5-6|
7| ….txt · Text · 5 lines · 3:7

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:99:99
size 30 8

--- past the end: the end of the last line
0|1 one
1|2 two
2|3     three
3|4 four
4|5 five
 |------^-----------------------
5-6|
7| ….txt · Text · 5 lines · 5:5

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:2-4
size 30 8

--- lines 2 to 4 selected with their breaks, the cursor at the start of line 2
0|1 one
1|2 two
 |  ^~~~
2|3     three
 |  ~~~~~~~~~~
3|4 four
 |  ~~~~~
4|5 five
5-6|
7| ….txt · Text · 5 lines · 2:1

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:4-99
size 30 8

--- a range running past the end: line 4 to the end of the text
0|1 one
1|2 two
2|3     three
3|4 four
 |  ^~~~~
4|5 five
 |  ~~~~
5-6|
7| ….txt · Text · 5 lines · 4:1

%reset
%file main.txt "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12"
%start main.txt:6-12
size 30 6

--- a range taller than the screen is shown from its first line
0| 4 4
1| 5 5
2| 6 6
 |   ^~
3| 7 7
 |   ~~
4| 8 8
 |   ~~
5| …txt · Text · 12 lines · 6:1

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:99-120
size 30 8

--- a range wholly past the end selects nothing: the cursor at the end of the text
0|1 one
1|2 two
2|3     three
3|4 four
4|5 five
 |------^-----------------------
5-6|
7| ….txt · Text · 5 lines · 5:5

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:3:4:
size 30 8

--- a colon after the place, as a compiler writes it, changes nothing
0|1 one
1|2 two
2|3     three
 |--------^---------------------
3|4 four
4|5 five
5-6|
7| ….txt · Text · 5 lines · 3:7

%reset
%file main.txt "one\ntwo\n\tthree\nfour\nfive"
%start main.txt:99999999999999999999
size 30 8

--- a number too large to hold is past the end like any other: the last line
0|1 one
1|2 two
2|3     three
3|4 four
4|5 five
 |--^---------------------------
5-6|
7| ….txt · Text · 5 lines · 5:1

%reset
%file main.txt "one\ntwo"
%file main.txt:2 "a file of that name"
%start main.txt:2
size 30 8

--- a file called `main.txt:2` is there, and is the one opened
0|1 a file of that name
 |--^---------------------------
1-6|
7| …xt:2 · Text · 1 lines · 1:1

%reset
%file a:1 "x\ny\nz"
%start a:1:2
size 30 8

--- `a:1` is there and `a` is not: line 2 of `a:1`, not line 1 column 2 of `a`
0|1 x
1|2 y
 |--^---------------------------
2|3 z
3-6|
7|   a:1 · Text · 3 lines · 2:1

%reset
%start new.txt:7
size 30 8

--- no such file under either name: a new one called `new.txt`
0|1
 |--^---------------------------
1-6|
7| ….txt · Text · 1 lines · 1:1

