# Opening a file with a grammar does not wait for its syntax tree. The
# same text is opened as plain text and as Rust, in sizes from sixteen
# kilobytes to four megabytes. Reading it takes as long as it is large either way. What
# the editor's own thread takes for the Rust file over what it takes for
# the plain one must not grow with the size: the first parse is a
# background job, and the file is shown before it is back. Parsing where
# the file is opened grows as the size and fails.
#
# The text comes through a named pipe, so that nothing of that size is
# written to the disk.

%for SIZE 16K 64K 256K 1M 4M
%shell mkfifo big.txt; (yes 'fn f(a: u32) -> u32 { let x = "s"; a + 1 }' | head -c $SIZE > big.txt &)
%measure plain
%start big.txt
?blocked plain linear

%reset
%shell mkfifo big.rs; (yes 'fn f(a: u32) -> u32 { let x = "s"; a + 1 }' | head -c $SIZE > big.rs &)
%measure rust
%start big.rs
?blocked rust constant over plain
# The parse, so that it is not left running into the next pass.
wait
