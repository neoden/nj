# The history has two ends. Past the newest edit there is nothing to redo;
# before the oldest, nothing to undo: Ctrl+Y and Ctrl+Z there change
# nothing. Undone all the way, the buffer is as it began, with nothing to
# mark as changed.

size 30 5
type "hello"

--- one word typed
0|1 hello
 |-------^----------------------
1-3|
4| …ch]* · Text · 1 lines · 1:6

key Ctrl+Y

--- nothing has been undone: nothing to redo
same

key Ctrl+Z

--- the word taken back: the buffer as it began, unmarked
0|1
 |--^---------------------------
4| …tch] · Text · 1 lines · 1:1

key Ctrl+Z

--- nothing left to undo
same

