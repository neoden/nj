# Typing in plain text costs what is typed, not what the file holds. A
# letter, a line break, a deletion and their undo are done at the start
# and at the end of files of short lines from two kilobytes to twenty
# megabytes. Their memory must not grow with the size, and the time they
# take no faster than its logarithm, which is finding the place in the
# tree the text is kept in. A copy of the text, or a walk over it, grows
# as the size and fails.
#
# Plain text only: the syntax tree of a file with a grammar is brought up
# to date after every edit, which costs as the size of the file, and that
# is not what this checks.
#
# The text comes through a named pipe, so that nothing of that size is
# written to the disk.

%for SIZE 2K 20K 200K 2M 20M
%shell mkfifo big.txt; (yes 'the quick brown fox jumps over the lazy dog' | head -c $SIZE > big.txt &)
%start big.txt
size 40 6

%measure at_start
type "x"
key Enter
key Backspace
key Ctrl+Z
key Ctrl+Z
key Ctrl+Z
?memory at_start constant
?time at_start log

key Ctrl+End
%measure at_end
type "x"
key Enter
key Backspace
key Ctrl+Z
key Ctrl+Z
key Ctrl+Z
?memory at_end constant
?time at_end log
