# With soft wrap on, moving the cursor in a file that is one long line
# costs what the screen holds, not what the line holds. The jump to the
# end of the line, and the steps and the jump back from there, are done in
# lines from a kilobyte to a hundred and eighty long. Their memory must
# not grow with the length, and the time they take no faster than its
# logarithm. Laying the whole line out to learn where its rows start, at
# every step over a row, grows as the length and fails.
#
# The line comes through a named pipe, so that nothing of that size is
# written to the disk.

%for LENGTH 1K 6K 32K 180K
%shell mkfifo long.txt; (head -c $LENGTH /dev/zero | tr '\0' x > long.txt &)
%start long.txt
size 40 6
key Alt+z

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
