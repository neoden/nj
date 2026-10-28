# What Ctrl+F puts in the field: the selected text, when it lies within one
# line, else the last query, else nothing. Either comes selected, so the
# next keystroke replaces it and Enter takes it as it is. The word under
# the cursor is not offered: a cursor is not a selection. Opening moves
# nothing in the text.

size 36 7
paste "alpha beta gamma\nbeta"
key Ctrl+Home
key Shift+Down
key Ctrl+F

--- a selection across lines, no search yet: the field is empty
0|1 alpha beta gamma
 |  ~~~~~~~~~~~~~~~~~
1|2 beta
 |  ^
2|
3|────────────────────────────────────
4| Find:
 |       ^
5|────────────────────────────────────
6|  [scratch]* · Text · 2 lines · 2:1

key Esc
key Ctrl+Right
key Shift+Ctrl+Left
key Ctrl+F

--- `beta` is selected: the field starts with it, selected
0|1 alpha beta gamma
1|2 beta
 |  ^~~~
4| Find: beta
 |       ~~~~^

key Enter
key Esc
key Ctrl+End
key Ctrl+F

--- the search is over and nothing is selected: the last query, selected
1|2 beta
 |::----^-----------------------------
6|  [scratch]* · Text · 2 lines · 2:5

type "ga"

--- typing replaces it
4| Find: ga
 |         ^

key Enter

--- found from the cursor, wrapping
0|1 alpha beta gamma
 |             ~~^
1|2 beta
3-5|
6| 1/1 "ga" … · Text · 2 lines · 1:14

key Ctrl+Home
key Shift+Down
key Ctrl+F

--- a selection across lines does not seed: the last query again
0|1 alpha beta gamma
 |  ~~~~~~~~~~~~~~~~~
1|2 beta
 |  ^
3|────────────────────────────────────
4| Find: ga
 |       ~~^
5|────────────────────────────────────
6| 1 "ga" …h]* · Text · 2 lines · 2:1

