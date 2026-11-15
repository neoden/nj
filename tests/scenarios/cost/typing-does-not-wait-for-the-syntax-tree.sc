# Typing in a file with a grammar does not wait for its syntax tree. The
# tree is parsed again after an edit, and that takes as long as the file
# is large, but it is a background job: the editor's own thread only
# tells the old tree where things have moved to, and draws from that. A
# letter and its deletion are done at the end of Rust files from two
# kilobytes to four megabytes. The time the editor's thread is busy must
# grow no faster than the logarithm of the size. All the time taken, the
# jobs' with it, may grow as the size and no faster.
#
# At the end of the file, because a frame at the start of a large file
# with a grammar has a cost of its own that grows with the size:
# `the-frame-does-not-depend-on-the-size-of-a-file-with-a-grammar`.
#
# The text comes through a named pipe, so that nothing of that size is
# written to the disk.

%for SIZE 2K 16K 128K 1M 4M
%shell mkfifo big.rs; (yes 'fn f(a: u32) -> u32 { let x = "s"; a + 1 }' | head -c $SIZE > big.rs &)
%start big.rs
size 40 6
key Ctrl+End
# The first parse, of the file as it was opened. Then a step each way
# that is not measured: the first frames after a wait vary from pass to
# pass.
wait
key Left
key Right

%measure typing
type "x"
key Backspace
?blocked typing log
wait
?time typing linear
