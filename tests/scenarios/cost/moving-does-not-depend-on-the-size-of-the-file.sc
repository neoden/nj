# Moving the cursor and drawing the frame after it cost what the screen
# holds, not what the file holds. A step to the side, a step down, a page
# and a jump to either end are done in files of short lines from two
# kilobytes to twenty megabytes. Their memory must not grow with the size,
# and the time they take no faster than its logarithm, which is finding a
# line in the tree the text is kept in. A copy of the text, or a walk over
# it, grows as the size and fails.
#
# The text comes through a named pipe, so that nothing of that size is
# written to the disk.

%for SIZE 2K 20K 200K 2M 20M
%shell mkfifo big.txt; (yes 'the quick brown fox jumps over the lazy dog' | head -c $SIZE > big.txt &)
%start big.txt
size 40 6

%measure move
key Right
key Down
key PageDown
key Ctrl+End
key Ctrl+Home
?memory move constant
?time move log
