# Ctrl+Home goes to the start of the text and Ctrl+End to its end, from
# anywhere, scrolling the view along so the cursor is in it. With Shift
# they select the way there.

size 30 5
paste "one\ntwo\nthree\nfour\nfive\nsix"

--- a paste leaves the cursor at the end; the view shows the last lines
0|3 three
1|4 four
2|5 five
3|6 six
 |-----^------------------------
4| …ch]* · Text · 6 lines · 6:4

key Ctrl+Home

--- Ctrl+Home: the first line, and the view scrolled up to it
0|1 one
 |--^---------------------------
1|2 two
2|3 three
3|4 four
4| …ch]* · Text · 6 lines · 1:1

key Down
key Right

--- a cursor in the middle of the second line
0|1 one
1|2 two
 |---^--------------------------
4| …ch]* · Text · 6 lines · 2:2

key Ctrl+Shift+End

--- Ctrl+Shift+End selects from there to the end of the text
0|3 three
 |  ~~~~~~
1|4 four
 |  ~~~~~
2|5 five
 |  ~~~~~
3|6 six
 |  ~~~^
4| …ch]* · Text · 6 lines · 6:4

key Ctrl+End

--- Ctrl+End alone collapses the selection at the end
0|3 three
1|4 four
2|5 five
3|6 six
 |-----^------------------------

