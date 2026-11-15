# Drawing the frame of a file with a grammar costs what the screen holds,
# not what the file holds. The cursor is moved a step each way, with no
# edit, at the start and at the end of Rust files from two kilobytes to
# eight megabytes. The memory must not grow with the size, and the time
# no faster than its logarithm.
#
# Reading the colours of a row off the syntax tree takes longer the more
# of the tree lies after the row: at the end of the file it does not
# grow, at the start it grows with the size and fails.
#
# The text comes through a named pipe, so that nothing of that size is
# written to the disk.

%for SIZE 2K 20K 200K 2M 8M
%shell mkfifo big.rs; (yes 'fn f(a: u32) -> u32 { let x = "s"; a + 1 }' | head -c $SIZE > big.rs &)
%start big.rs
size 40 6
# The first parse: until it is back there are no colours to read. Then a
# step each way that is not measured: the first frames after a wait vary
# by a tenth of a millisecond from pass to pass, which is as much as the
# growth looked for.
wait
key Right
key Left

%measure at_start
key Right
key Down
key Left
key Up
?memory at_start constant
?time at_start log

key Ctrl+End
%measure at_end
key Left
key Up
key Right
key Down
?memory at_end constant
?time at_end log
