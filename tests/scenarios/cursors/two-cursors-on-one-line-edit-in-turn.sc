# Two cursors on one line edit the same text, and what the first inserts
# moves the second along, so both stay on the text they were put at.
# Alt+click adds a cursor where the mouse is. Typing, Backspace and Tab
# apply at each in turn; Tab pads each cursor to the next stop from where
# it stands after the cursors before it have had their say.

size 30 5
type "one two three"
key Home
key Ctrl+Right
click 9 0 Alt

--- two cursors: after `one` and after `two`
0|1 one two three
 |     ^   ^
1-3|
4| 2 cursors …t · 1 lines · 1:4

type ","

--- a comma at each; the second cursor moved along with the first comma
0|1 one, two, three
 |      ^    ^
4| 2 cursors …t · 1 lines · 1:5

key Backspace

--- both commas gone
0|1 one two three
 |     ^   ^
4| 2 cursors …t · 1 lines · 1:4

key Tab

--- each cursor at a stop: the first padded one column, the second four, from where it stood after the first
0|1 one  two     three
 |      ^       ^
4| 2 cursors …t · 1 lines · 1:5

