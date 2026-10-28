# Opening the find field starts nothing and ends nothing: only Enter puts
# a search in place of the one going on. A field wiped and closed with
# Esc started nothing, so the search there was goes on, and F3 walks it.

size 36 6
paste "ga ga"
key Ctrl+Home
key Ctrl+F
type "ga"
key Enter

--- a search for `ga`, on its first hit
0|1 ga ga
 |  ~~^==
1-4|
5| 1/2 "ga" …* · Text · 1 lines · 1:3

key Ctrl+F
key Backspace
key Esc

--- the field came back with the query, was wiped and closed: the search is still on
same

key F3

--- and F3 walks it to the second hit
0|1 ga ga
 |  == ~~^
5| 2/2 "ga" …* · Text · 1 lines · 1:6

