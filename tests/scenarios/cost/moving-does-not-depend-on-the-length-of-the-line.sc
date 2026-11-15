# Moving the cursor in a file that is one long line costs what the screen
# holds, not what the line holds. The jump to the end of the line, and the
# steps and the jump back from there, are done in lines from a kilobyte to
# a megabyte long. Their memory must not grow with the length, and the
# time they take no faster than its logarithm. Counting the columns from
# the start of the line to the cursor grows as the length and fails.
#
# The line comes through a named pipe, so that nothing of that size is
# written to the disk.

%for LENGTH 1K 6K 32K 180K 1M
%shell mkfifo long.txt; (head -c $LENGTH /dev/zero | tr '\0' x > long.txt &)
%start long.txt
size 40 6

%measure to_end
key End
?memory to_end constant
?time to_end log

%measure at_end
key Left
key Left
key Right
key Up
key Down
?memory at_end constant
?time at_end log

%measure to_start
key Home
?memory to_start constant
?time to_start log
