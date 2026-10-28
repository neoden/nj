# A search going on belongs to the text, not to the field that started it.
# Another prompt over it — Ctrl+G here — neither ends it nor searches for
# what is typed into it, and F3 in that prompt does nothing: the prompt has
# no hits to walk, and the search behind it is not touched either. Once
# the prompt is closed, F3 walks the search as before.

size 30 7
paste "1 x 1\n2 x 2\n3 x 3"
key Ctrl+Home
key Ctrl+F
type "x"
key Enter

--- the search is on: the first `x` selected, the others lit
0|1 1 x 1
 |    ~^
1|2 2 x 2
 |    =
2|3 3 x 3
 |    =
3-5|
6| 1/3 "x" …ext · 3 lines · 1:4

key Ctrl+G
type "1"

--- Ctrl+G over the search: the `1`s are not hits, the `x`s still are
3|──────────────────────────────
4| Line: 1
 |        ^
5|──────────────────────────────

key F3

--- F3 in the line prompt: nothing moves, the prompt stays
same

key Esc

--- the prompt closed, the search as it was
3-5|

key F3

--- F3 walks the search again
0|1 1 x 1
 |    =
1|2 2 x 2
 |    ~^
6| 2/3 "x" …ext · 3 lines · 2:4

