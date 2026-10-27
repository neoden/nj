# One keystroke goes to every selection and every bare cursor alike: typing
# replaces what is selected and inserts at a cursor, in the same edit;
# Backspace removes a selection whole and one grapheme at a cursor; Enter
# splits at each, copying the indent of each line. One undo takes the lot
# back.

size 30 7
paste "one two\n    three four"
key Ctrl+Home
key Ctrl+D
click 11 1 Alt

--- `one` selected on the first line, a bare cursor after `three` on the second
0|1 one two
 |  ~~~^
1|2     three four
 |           ^
2-5|
6| 2 cursors …t · 2 lines · 1:4

type "X"

--- `one` replaced, `X` inserted after `three`, in one go
0|1 X two
 |   ^
1|2     threeX four
 |            ^
6| 2 cursors …t · 2 lines · 1:2

key Ctrl+Z

--- one undo takes both back, and the selection with them
0|1 one two
 |  ~~~^
1|2     three four
 |           ^
6| 2 cursors …t · 2 lines · 1:4

key Backspace

--- Backspace: the selection goes whole, the cursor takes the `e`
0|1  two
 |  ^
1|2     thre four
 |          ^
6| 2 cursors …t · 2 lines · 1:1

key Enter

--- Enter at each: the second line's new line keeps its indent, the first's has none
0|1
1|2  two
 |  ^
2|3     thre
3|4      four
 |      ^
6| 2 cursors …t · 4 lines · 2:1

key Ctrl+Z

--- one undo, both splits gone
0|1  two
 |  ^
1|2     thre four
 |          ^
2-3|
6| 2 cursors …t · 2 lines · 1:1

