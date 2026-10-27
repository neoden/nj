# Ctrl+D with several bare cursors takes the word under each, so the
# selections may come to hold different texts. From there Ctrl+D has no
# one text to look for and does nothing, rather than picking one of them;
# Ctrl+Shift+L is the same. Once the selections agree again, Ctrl+D goes
# on as usual.

size 30 7
paste "cat dog\ncat dog\ncat"
key Ctrl+Home
click 7 0 Alt
key Ctrl+D

--- two cursors on `cat` and `dog`: Ctrl+D takes a word each
0|1 cat dog
 |  ~~~^~~~^
1|2 cat dog
2|3 cat
3-5|
6| 2 cursors …t · 3 lines · 1:4

key Ctrl+D

--- the selections differ: nothing happens
same

key Ctrl+Shift+L

--- nor for Ctrl+Shift+L
same

key Esc
key Esc
key Ctrl+D
key Ctrl+D

--- back to one cursor, Ctrl+D twice: `cat` and the next `cat`
0|1 cat dog
 |  ~~~^
1|2 cat dog
 |  ~~~^

