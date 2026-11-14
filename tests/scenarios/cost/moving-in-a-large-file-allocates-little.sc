# Moving the cursor and drawing the frame after it cost what the screen
# holds, not what the file holds: in twenty megabytes of short lines a step
# to the side, a step down, a page and a jump to the far end each allocate
# some kilobytes with the frame drawn after them. A copy of the text, or of
# anything that grows with it, would be megabytes.
#
# The text comes through a named pipe, so that nothing of that size is
# written to the disk.

%shell mkfifo big.txt; (yes 'the quick brown fox jumps over the lazy dog' | head -c 20000000 > big.txt &)
%start big.txt
size 40 6

--- the file is open
0|     1 the quick brown fox jumps over th
 |-------^--------------------------------
1|     2 the quick brown fox jumps over th
2|     3 the quick brown fox jumps over th
3|     4 the quick brown fox jumps over th
4|     5 the quick brown fox jumps over th
5|    big.txt · Text · 454546 lines · 1:1

%allocated right
key Right
?allocated right 64K

--- a step to the side
0|     1 the quick brown fox jumps over th
 |--------^-------------------------------
5|    big.txt · Text · 454546 lines · 1:2

%allocated down
key Down
?allocated down 64K

--- a step down
0|     1 the quick brown fox jumps over th
1|     2 the quick brown fox jumps over th
 |--------^-------------------------------
5|    big.txt · Text · 454546 lines · 2:2

%allocated page
key PageDown
?allocated page 64K

--- a page down
0|     4 the quick brown fox jumps over th
1|     5 the quick brown fox jumps over th
2|     6 the quick brown fox jumps over th
 |--------^-------------------------------
3|     7 the quick brown fox jumps over th
4|     8 the quick brown fox jumps over th
5|    big.txt · Text · 454546 lines · 6:2

%allocated end
key Ctrl+End
?allocated end 64K

--- the far end
0|454542 the quick brown fox jumps over th
1|454543 the quick brown fox jumps over th
2|454544 the quick brown fox jumps over th
3|454545 the quick brown fox jumps over th
4|454546 the quick brown fox
 |---------------------------^------------
5| …txt · Text · 454546 lines · 454546:21

