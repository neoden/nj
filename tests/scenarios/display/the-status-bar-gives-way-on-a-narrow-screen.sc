# The status bar has two halves: the document on the right — name, language,
# line count, line and column — and on the left whatever is going on, a
# message or a count of cursors. When the screen is too narrow for both,
# the document gives way first, and it is cut from its start, so that the
# line and column at its end stay in sight. While the left has something to
# say, the document keeps half the screen and the message takes the rest.

%file a-rather-long-file-name.txt "text"
%start a-rather-long-file-name.txt
size 60 3

--- room for everything
0|1 text
 |--^---------------------------------------------------------
1|
2|         a-rather-long-file-name.txt · Text · 1 lines · 1:1

size 40 3

--- too narrow for the name: it loses its start, the line and column stay
0|1 text
 |--^-------------------------------------
1|
2| …-file-name.txt · Text · 1 lines · 1:1

key Ctrl+O
type "nowhere.txt"
key Enter

--- a message: the document gives up all but half the screen to it
2| cannot read nowher …xt · 1 lines · 1:1

size 24 3

--- narrower still: the message is cut at its end, the document at its start
0|1 text
 |--^---------------------
1|
2| cannot rea …ines · 1:1

size 12 3

--- barely a screen: the line and column are the last to go
0|1 text
 |--^---------
1|
2| cann … 1:1

