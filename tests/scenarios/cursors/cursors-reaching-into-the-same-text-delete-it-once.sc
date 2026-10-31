# Two cursors in one word both reach for its edge, and their reaches
# overlap. What goes is the union, taken once: the same bytes are not
# deleted twice, which would eat into what lies beyond. The cursors land
# together where the deletion ends. The same holds for Backspace inside an
# indent, where each cursor reaches back to a tab stop.

size 30 4
paste "abcd ef"
key Home
key Right
key Right
click 5 0 Alt

--- two cursors inside `abcd`, after `b` and after `c`
0|1 abcd ef
 |    ^^
1-2|
3| 2 cursors …t · 1 lines · 1:3

key Ctrl+Backspace

--- Ctrl+Backspace: `abc` gone once, `d ef` stays, one cursor at the start
0|1 d ef
 |--^---------------------------
3| …ch]* · Text · 1 lines · 1:1

key Ctrl+A
paste "abcd ef"
key Home
click 3 0 Alt

--- again, cursors before `a` and after it
0|1 abcd ef
 |  ^^
3| 2 cursors …t · 1 lines · 1:1

key Ctrl+Delete

--- Ctrl+Delete: `abcd` gone once, the space and `ef` stay
0|1  ef
 |--^---------------------------
3| …ch]* · Text · 1 lines · 1:1

key Ctrl+A
paste "      x"
click 4 0
click 6 0 Alt

--- six spaces before `x`, cursors at columns 2 and 4
0|1       x
 |    ^ ^
3| 2 cursors …t · 1 lines · 1:3

key Backspace

--- each reaches back to a stop, 0 for both: four spaces gone, two remain before `x`
0|1   x
 |--^---------------------------
3| …ch]* · Text · 1 lines · 1:1

