# A question is answered by the key, not the letter: on a Cyrillic layout
# `й` sits where `q` does, so it quits, and `ц`, where `w` is, answers
# nothing, like `w`.

size 40 9
type "text"
key Ctrl+Q

--- the question
0|1 text
 |::----^---------------------------------
1|
2|────────────────────────────────────────
3| Unsaved changes
4|   s     Save and quit
5|   q     Quit anyway
6|   esc   Cancel
7|────────────────────────────────────────
8|      [scratch]* · Text · 1 lines · 1:5

type "ц"

--- a letter whose key is not an answer: the question stands
same

type "й"
?exit 0
