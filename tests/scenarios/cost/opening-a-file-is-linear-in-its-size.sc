# Opening a file has to read all of it, so its memory and the time it
# takes grow as the size of the file, and must grow no faster: reading the
# text over again for every line of it would. The sizes go from sixteen
# kilobytes to sixty-four megabytes.

%for SIZE 16K 128K 1M 8M 64M
%shell mkfifo big.txt; (yes 'the quick brown fox jumps over the lazy dog' | head -c $SIZE > big.txt &)

%measure open
%start big.txt
?memory open linear
?time open linear
