# A query with no hit has nothing to ask about: the form closes, the status
# says not found, and the text and the cursor are left as they were.

size 40 7
paste "cat dog"
key Ctrl+Home
key Ctrl+H
type "cow"
key Tab
type "bird"
key Enter

--- not found; no question, nothing changed
0|1 cat dog
 |--^-------------------------------------
1-5|
6| Not found …ch]* · Text · 1 lines · 1:1

