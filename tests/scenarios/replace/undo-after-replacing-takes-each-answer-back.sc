# Every `y` is an edit of its own, so after a walk each Ctrl+Z takes one
# replacement back, the latest first, and the cursor goes to it. `a` is one
# edit for all it replaced, and one Ctrl+Z takes all of it back.

size 72 7
paste "cat dog\ncat\ndog cat\ncat"
key Ctrl+Home
key Ctrl+H
type "cat"
key Tab
type "bird"
key Enter
type "y"
type "y"
key Esc

--- two replaced one by one, then the walk stopped
0|1 bird dog
1|2 bird
2|3 dog cat
 |      ~~~^
3|4 cat
4-5|
6| Replaced 2                           [scratch]* · Text · 4 lines · 3:8

key Ctrl+Z

--- the first undo takes the second replacement back
1|2 cat
 |  ~~~^
2|3 dog cat
6|                                      [scratch]* · Text · 4 lines · 2:4

key Ctrl+Z

--- the second takes the first
0|1 cat dog
 |  ~~~^
1|2 cat
6|                                      [scratch]* · Text · 4 lines · 1:4

key Ctrl+Y
key Ctrl+Y
key Ctrl+H
key Enter
type "a"

--- redone, then `a` over the two left: one edit
0|1 bird dog
1|2 bird
2|3 dog bird
 |          ^
3|4 bird
 |      ^
6| Replaced 2                           [scratch]* · Text · 4 lines · 3:9

key Ctrl+Z

--- one undo takes both of those back, and not the two replaced by `y`
2|3 dog cat
 |      ~~~^
3|4 cat
 |  ~~~^
6| 2 cursors                            [scratch]* · Text · 4 lines · 3:8

